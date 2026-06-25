#!/usr/bin/env python3
"""Black-box tests for build-index.py — run the script against a temp vault."""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
BUILDER = HERE / "build-index.py"


class IndexBuilderTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.home = Path(self.tmp)
        self.vault = self.home / "journal"
        for d in ("daily", "sessions", "notes", "mocs", ".index"):
            (self.vault / d).mkdir(parents=True, exist_ok=True)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    # Characters that require quoting a scalar YAML value in the test helper.
    _YAML_QUOTE_CHARS = set('[]{}#:\t\n\r"\'')

    def _yaml_scalar(self, val: str) -> str:
        """Return a YAML-safe representation of a scalar string value."""
        needs_quoting = (
            any(c in val for c in self._YAML_QUOTE_CHARS)
            or val.startswith(" ")
            or val.endswith(" ")
        )
        if needs_quoting:
            return '"%s"' % val.replace("\\", "\\\\").replace('"', '\\"')
        return val

    def write_note(self, sub, name, frontmatter, body=""):
        lines = ["---"]
        for key, val in frontmatter.items():
            if isinstance(val, list):
                # Quote items containing wikilink brackets or other special
                # characters so PyYAML does not misparse them.
                quoted = [self._yaml_scalar(item) for item in val]
                lines.append("%s: [%s]" % (key, ", ".join(quoted)))
            else:
                lines.append("%s: %s" % (key, self._yaml_scalar(str(val))))
        lines.append("---")
        text = "\n".join(lines) + "\n" + body
        (self.vault / sub / name).write_text(text, encoding="utf-8")

    def write_raw(self, sub, name, text):
        (self.vault / sub / name).write_text(text, encoding="utf-8")

    def run_builder(self, *args):
        env = dict(os.environ, AIRSSTACK_HOME=str(self.home))
        return subprocess.run(
            [sys.executable, str(BUILDER), *args],
            env=env, capture_output=True, text=True,
        )

    def read_idx(self, name):
        return (self.vault / ".index" / name).read_text(encoding="utf-8")

    def graph(self):
        return json.loads(self.read_idx("graph.json"))

    def tags(self):
        return json.loads(self.read_idx("tags.json"))

    def tsv_rows(self):
        text = self.read_idx("summaries.tsv")
        return [r.split("\t") for r in text.splitlines()]

    def index(self):
        return json.loads(self.read_idx("index.json"))

    def test_index_node_carries_type_and_path(self):
        self.write_note("notes", "alpha.md", {
            "title": "Alpha", "type": "insight", "summary": "a",
            "tags": ["tokio"], "domains": ["async-rust"],
            "project": "clauders", "helped": "2",
            "updated": "2026-06-23 10:00",
        })
        self.run_builder()
        node = self.index()["nodes"]["alpha"]
        self.assertEqual(node["type"], "insight")
        self.assertEqual(node["path"], "notes/alpha.md")
        self.assertEqual(node["helped"], 2)
        self.assertEqual(node["tags"], ["tokio"])
        self.assertEqual(node["domains"], ["async-rust"])

    def test_session_source_edge_is_contains(self):
        self.write_note("notes", "child.md", {"title": "Child", "summary": "c"})
        self.write_note("sessions", "session-ab12cd34.md",
                         {"title": "S", "type": "session", "summary": "s"},
                         body="spun off [[child]]")
        self.run_builder()
        self.assertIn(
            {"from": "session-ab12cd34", "to": "child", "type": "contains"},
            self.index()["edges"])

    def test_note_source_edge_is_references(self):
        self.write_note("notes", "target.md", {"title": "T", "summary": "t"})
        self.write_note("notes", "source.md",
                         {"title": "S", "type": "insight", "summary": "s"},
                         body="see [[target]]")
        self.run_builder()
        self.assertIn(
            {"from": "source", "to": "target", "type": "references"},
            self.index()["edges"])

    def test_backlinks_reverse_edges(self):
        self.write_note("notes", "target.md", {"title": "T", "summary": "t"})
        self.write_note("notes", "source.md", {"title": "S", "summary": "s"},
                         body="see [[target]]")
        self.run_builder()
        self.assertEqual(self.index()["backlinks"]["target"], ["source"])

    def test_index_unresolved_mirrors_graph(self):
        self.write_note("notes", "lonely.md", {"title": "L", "summary": "l"},
                         body="points at [[ghost]]")
        self.run_builder()
        self.assertIn(["lonely", "ghost"], self.index()["unresolved"])

    def test_empty_vault_yields_valid_empty_index(self):
        res = self.run_builder()
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertEqual(self.graph(), {})
        self.assertEqual(self.tags(), {})
        self.assertEqual(self.read_idx("summaries.tsv"), "")

    def test_summary_row_has_expected_columns(self):
        self.write_note("notes", "alpha.md", {
            "title": "Alpha note",
            "project": "clauders",
            "helped": "3",
            "updated": "2026-06-23 10:00",
            "summary": "alpha is the first",
        })
        self.run_builder()
        rows = self.tsv_rows()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0], [
            "alpha", "Alpha note", "alpha is the first",
            "clauders", "3", "2026-06-23 10:00",
        ])

    def test_project_list_is_joined_in_tsv(self):
        self.write_note("notes", "beta.md", {
            "title": "Beta",
            "project": ["clauders", "openrouter-rs"],
            "summary": "beta spans two",
        })
        self.run_builder()
        rows = self.tsv_rows()
        self.assertEqual(rows[0][3], "clauders, openrouter-rs")

    def test_tabs_and_newlines_neutralised_in_tsv(self):
        self.write_note("notes", "gamma.md", {
            "title": "Gamma",
            "summary": "has\ttab and stuff",
        })
        self.run_builder()
        text = self.read_idx("summaries.tsv")
        # exactly one row → exactly the column separators, no stray tabs
        self.assertEqual(len(text.splitlines()), 1)
        self.assertEqual(text.splitlines()[0].count("\t"), 5)

    def test_malformed_note_skipped_others_indexed(self):
        self.write_note("notes", "good.md", {"title": "Good", "summary": "ok"})
        self.write_raw("notes", "bad.md", "---\nthis line has no colon\n---\nbody")
        res = self.run_builder()
        self.assertEqual(res.returncode, 0)
        stems = [r[0] for r in self.tsv_rows()]
        self.assertIn("good", stems)
        self.assertNotIn("bad", stems)
        self.assertIn("bad", res.stderr)

    def test_tags_and_domains_inverted(self):
        self.write_note("notes", "delta.md", {
            "title": "Delta",
            "tags": ["tokio", "shutdown"],
            "domains": ["async-rust"],
            "summary": "d",
        })
        self.run_builder()
        tags = self.tags()
        self.assertEqual(tags["tokio"], ["delta"])
        self.assertEqual(tags["shutdown"], ["delta"])
        self.assertEqual(tags["async-rust"], ["delta"])

    def test_graph_resolves_body_and_frontmatter_links(self):
        self.write_note("notes", "target.md", {"title": "Target", "summary": "t"})
        self.write_note("notes", "source.md",
                         {"title": "Source", "summary": "s",
                          "links": ["[[target]]"]},
                         body="see also [[target]] for details")
        self.run_builder()
        graph = self.graph()
        self.assertEqual(graph["source"], ["target"])
        self.assertEqual(graph["target"], [])

    def test_link_resolution_is_case_insensitive(self):
        self.write_note("notes", "widget.md", {"title": "Widget", "summary": "w"})
        self.write_note("notes", "user.md", {"title": "User", "summary": "u"},
                         body="uses [[Widget]] here")
        self.run_builder()
        self.assertEqual(self.graph()["user"], ["widget"])

    def test_unresolved_link_recorded_not_fatal(self):
        self.write_note("notes", "lonely.md", {"title": "Lonely", "summary": "l"},
                         body="points at [[ghost]] which does not exist")
        res = self.run_builder()
        self.assertEqual(res.returncode, 0)
        graph = self.graph()
        self.assertEqual(graph["lonely"], [])
        self.assertIn(["lonely", "ghost"], graph["_unresolved"])

    def test_force_flag_accepted(self):
        self.write_note("notes", "x.md", {"title": "X", "summary": "x"})
        res = self.run_builder("--force")
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertIn("x", [r[0] for r in self.tsv_rows()])

    def test_rebuild_is_deterministic(self):
        self.write_note("notes", "a.md", {"title": "A", "tags": ["t1"],
                                           "summary": "a", "links": ["[[b]]"]})
        self.write_note("notes", "b.md", {"title": "B", "tags": ["t2"],
                                           "summary": "b"})
        self.run_builder()
        first = (self.read_idx("graph.json"), self.read_idx("tags.json"),
                 self.read_idx("summaries.tsv"))
        self.run_builder()
        second = (self.read_idx("graph.json"), self.read_idx("tags.json"),
                  self.read_idx("summaries.tsv"))
        self.assertEqual(first, second)


    def test_supersedes_emits_typed_edge(self):
        self.write_note("notes", "auth-v1.md", {"title": "v1", "summary": "old"})
        self.write_note("notes", "auth-v2.md",
                         {"title": "v2", "summary": "new",
                          "supersedes": ["[[auth-v1]]"]})
        self.run_builder()
        self.assertIn({"from": "auth-v2", "to": "auth-v1", "type": "supersedes"},
                      self.index()["edges"])

    def test_depends_on_emits_typed_edge(self):
        self.write_note("notes", "base.md", {"title": "B", "summary": "b"})
        self.write_note("notes", "derived.md",
                         {"title": "D", "summary": "d",
                          "depends-on": ["[[base]]"]})
        self.run_builder()
        self.assertIn({"from": "derived", "to": "base", "type": "depends-on"},
                      self.index()["edges"])

    def test_typed_edge_takes_precedence_over_reference(self):
        self.write_note("notes", "old.md", {"title": "O", "summary": "o"})
        self.write_note("notes", "new.md",
                         {"title": "N", "summary": "n",
                          "links": ["[[old]]"], "supersedes": ["[[old]]"]},
                         body="see [[old]]")
        self.run_builder()
        pair = [e for e in self.index()["edges"]
                if e["from"] == "new" and e["to"] == "old"]
        self.assertEqual(pair, [{"from": "new", "to": "old", "type": "supersedes"}])

    def test_dangling_typed_target_is_unresolved(self):
        self.write_note("notes", "x.md", {"title": "X", "summary": "x",
                                          "depends-on": ["[[ghost]]"]})
        self.run_builder()
        self.assertIn(["x", "ghost"], self.index()["unresolved"])
        self.assertEqual(
            [e for e in self.index()["edges"] if e["to"] == "ghost"], [])

    def test_typed_edge_appears_in_backlinks(self):
        self.write_note("notes", "auth-v1.md", {"title": "v1", "summary": "old"})
        self.write_note("notes", "auth-v2.md",
                         {"title": "v2", "summary": "new",
                          "supersedes": ["[[auth-v1]]"]})
        self.run_builder()
        self.assertEqual(self.index()["backlinks"]["auth-v1"], ["auth-v2"])

    def test_self_link_in_body_does_not_self_loop(self):
        # A note that mentions its own stem in prose must not create a
        # self-edge or self-backlink.
        self.write_note("sessions", "session-ab12cd34.md",
                        {"title": "S", "type": "session", "summary": "s"},
                        body="appended [[session-ab12cd34]] to the daily note")
        self.run_builder()
        idx = self.index()
        self.assertEqual(
            [e for e in idx["edges"] if e["from"] == e["to"]], [])
        self.assertNotIn("session-ab12cd34", idx["backlinks"])
        self.assertEqual(self.graph()["session-ab12cd34"], [])

    def test_inline_code_wikilink_is_not_an_edge(self):
        # A [[target]] inside an inline-code span is prose ABOUT a link,
        # not a real link — it must not create an edge.
        self.write_note("notes", "target.md", {"title": "T", "summary": "t"})
        self.write_note("notes", "doc.md", {"title": "D", "summary": "d"},
                        body="write the `[[target]]` form in backticks")
        self.run_builder()
        self.assertEqual(self.graph()["doc"], [])
        self.assertEqual(
            [e for e in self.index()["edges"] if e["from"] == "doc"], [])

    def test_fenced_code_wikilink_is_not_an_edge(self):
        self.write_note("notes", "target.md", {"title": "T", "summary": "t"})
        self.write_note("notes", "doc.md", {"title": "D", "summary": "d"},
                        body="example:\n```\nsee [[target]] here\n```\ndone")
        self.run_builder()
        self.assertEqual(self.graph()["doc"], [])

    def test_real_link_outside_code_still_resolves(self):
        # Guard: stripping code must not eat genuine prose links.
        self.write_note("notes", "target.md", {"title": "T", "summary": "t"})
        self.write_note("notes", "doc.md", {"title": "D", "summary": "d"},
                        body="real [[target]] and a `[[target]]` mention")
        self.run_builder()
        self.assertEqual(self.graph()["doc"], ["target"])

    def test_supersedes_absent_from_graph_json(self):
        self.write_note("notes", "auth-v1.md", {"title": "v1", "summary": "old"})
        self.write_note("notes", "auth-v2.md",
                         {"title": "v2", "summary": "new",
                          "supersedes": ["[[auth-v1]]"]})
        self.run_builder()
        # graph.json stays untyped adjacency: a supersedes-only target is NOT a graph edge.
        self.assertEqual(self.graph()["auth-v2"], [])

    def test_block_style_depends_on_emits_typed_edges(self):
        """Block-style YAML list in depends-on must produce typed edges (Obsidian compat)."""
        self.write_note("notes", "session-5481f011.md",
                        {"title": "S1", "summary": "s1"})
        self.write_note("notes", "session-e06b6653.md",
                        {"title": "S2", "summary": "s2"})
        # Write raw so we control the exact YAML block-list syntax Obsidian emits.
        self.write_raw(
            "notes",
            "derived.md",
            (
                "---\n"
                "title: Derived\n"
                "summary: derived note\n"
                "depends-on:\n"
                '  - "[[session-5481f011]]"\n'
                '  - "[[session-e06b6653]]"\n'
                "---\n"
                "body text\n"
            ),
        )
        res = self.run_builder()
        self.assertEqual(res.returncode, 0, res.stderr)
        # Note must NOT be skipped as malformed.
        stems = [r[0] for r in self.tsv_rows()]
        self.assertIn("derived", stems)
        edges = self.index()["edges"]
        self.assertIn(
            {"from": "derived", "to": "session-5481f011", "type": "depends-on"},
            edges,
        )
        self.assertIn(
            {"from": "derived", "to": "session-e06b6653", "type": "depends-on"},
            edges,
        )

    def test_inline_flow_list_still_parses_after_yaml_migration(self):
        """Regression: inline [a, b] flow-style lists must still yield typed edges."""
        self.write_note("notes", "auth-v1.md", {"title": "v1", "summary": "old"})
        # write_note writes inline [a, b] style — ensure it still works.
        self.write_note(
            "notes",
            "auth-v2.md",
            {"title": "v2", "summary": "new", "supersedes": ["[[auth-v1]]"]},
        )
        res = self.run_builder()
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertIn(
            {"from": "auth-v2", "to": "auth-v1", "type": "supersedes"},
            self.index()["edges"],
        )


if __name__ == "__main__":
    unittest.main()
