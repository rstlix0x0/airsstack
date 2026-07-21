#!/usr/bin/env python3
"""Unit tests for the airsstack enforcement dispatcher's pure units.

Run: python3 -m unittest discover -s plugins/airsstack/hooks -p 'test_*.py'
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import enforce  # noqa: E402


class TestGlobToRegex(unittest.TestCase):
    # The spec's verified 11-case baseline (§11), plus character classes.
    CASES = [
        ("crates/**/*.rs", "crates/clauders/src/lib.rs", True),
        ("crates/**/*.rs", "crates/lib.rs", True),
        ("crates/**/*.rs", "src/x.rs", False),
        ("**/*.rs", "lib.rs", True),
        ("**/*.rs", "crates/clauders/src/lib.rs", True),
        ("**/*.rs", "src/x.rs", True),
        ("src/gen/*.rs", "src/gen/a.rs", True),
        ("src/gen/*.rs", "src/gen/nested/a.rs", False),
        ("**/Cargo.toml", "Cargo.toml", True),
        ("**/Cargo.toml", "crates/clauders/Cargo.toml", True),
        ("**/*.rs", "src/main.py", False),
        ("**/v[0-9].rs", "src/v3.rs", True),
        ("**/v[0-9].rs", "src/vx.rs", False),
        ("**/v[!0-9].rs", "src/vx.rs", True),
        ("**/v[!0-9].rs", "src/v3.rs", False),
        ("**/a?.rs", "src/ab.rs", True),
        ("**/a?.rs", "src/abc.rs", False),
    ]

    def test_baseline(self):
        for pattern, path, expected in self.CASES:
            got = bool(enforce.glob_to_regex(pattern).match(path))
            self.assertEqual(got, expected, "%r vs %r" % (pattern, path))

    def test_unclosed_bracket_is_literal(self):
        self.assertTrue(enforce.glob_to_regex("a[b.rs").match("a[b.rs"))

    def test_dot_is_not_a_wildcard(self):
        self.assertFalse(enforce.glob_to_regex("**/*.rs").match("src/xxrs"))


class TestProjectKey(unittest.TestCase):
    def test_matches_the_shell_formula(self):
        """Python key must equal the sh formula in references/artifact-paths.md."""
        import subprocess
        script = (
            'if common_dir=$(git rev-parse --git-common-dir 2>/dev/null); then\n'
            '  abs=$(cd "$(dirname "$common_dir")" 2>/dev/null && pwd -P)/$(basename "$common_dir")\n'
            '  base=$(basename "$(dirname "$abs")")\n'
            'else\n'
            '  abs=$(pwd -P); base=$(basename "$abs")\n'
            'fi\n'
            "base=$(printf '%s' \"$base\" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')\n"
            "hash8=$(printf '%s' \"$abs\" | shasum | cut -c1-8)\n"
            'printf "%s-%s" "$base" "$hash8"\n'
        )
        here = os.path.dirname(os.path.abspath(__file__))
        expected = subprocess.check_output(
            ["sh", "-c", script], cwd=here
        ).decode("utf-8").strip()
        self.assertEqual(enforce.project_key(here), expected)

    def test_no_git_falls_back_to_cwd_hash(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            key = enforce.project_key(tmp)
            self.assertIsNotNone(key)
            self.assertRegex(key, r"^[A-Za-z0-9._-]+-[0-9a-f]{8}$")


class TestPathForMatching(unittest.TestCase):
    def test_inside_repo_returns_repo_relative(self):
        here = os.path.dirname(os.path.abspath(__file__))
        target = os.path.join(here, "enforce.py")
        self.assertEqual(
            enforce.path_for_matching(target, here),
            "plugins/airsstack/hooks/enforce.py",
        )

    def test_outside_repo_falls_back_to_basename(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            target = os.path.join(tmp, "sub", "lib.rs")
            self.assertEqual(enforce.path_for_matching(target, tmp), "lib.rs")

    def test_matches_any_glob(self):
        self.assertTrue(
            enforce.matches_any("crates/clauders/src/lib.rs", ["**/Cargo.toml", "**/*.rs"])
        )
        self.assertFalse(enforce.matches_any("README.md", ["**/*.rs"]))


class TestDesignDoc(unittest.TestCase):
    def setUp(self):
        self.root = "/tmp/aihome"
        self.sdd = os.path.join(self.root, "cc", "plugins", "sdd")

    def check(self, rel):
        return enforce.is_design_doc(os.path.join(self.sdd, rel), self.root)

    def test_specs_segment_matches(self):
        self.assertTrue(self.check("proj/specs/2026-01-01-x.md"))

    def test_plans_segment_matches(self):
        self.assertTrue(self.check("proj/plans/2026-01-01-x.md"))

    def test_substring_only_rejects(self):
        self.assertFalse(self.check("proj/myspecs/x.md"))
        self.assertFalse(self.check("proj/specsheet/x.md"))

    def test_nested_accidental_substring_rejects(self):
        # The pre-repair '/specs/' in fp test matched this by accident.
        self.assertFalse(self.check("proj/a/specs/b/plans/c.md"))

    def test_outside_sdd_root_rejects(self):
        self.assertFalse(enforce.is_design_doc("/elsewhere/specs/x.md", self.root))


class TestMarkerActive(unittest.TestCase):
    def test_found_at_ancestor_of_the_file(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            repo = os.path.join(tmp, "repo", "src", "deep")
            os.makedirs(repo)
            open(os.path.join(tmp, "repo", "Cargo.toml"), "w").close()
            target = os.path.join(repo, "lib.rs")
            # cwd is deliberately elsewhere: the search must start at the
            # FILE's directory, not at the session's working directory.
            self.assertTrue(enforce.marker_active(target, ["Cargo.toml"], cwd=tmp))

    def test_absent_above_the_file(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            marked = os.path.join(tmp, "marked")
            plain = os.path.join(tmp, "plain")
            os.makedirs(marked)
            os.makedirs(plain)
            open(os.path.join(marked, "Cargo.toml"), "w").close()
            target = os.path.join(plain, "lib.rs")
            self.assertFalse(enforce.marker_active(target, ["Cargo.toml"], cwd=marked))

    def test_empty_marker_list_is_inactive(self):
        self.assertFalse(enforce.marker_active("/tmp/x/lib.rs", [], cwd="/tmp"))

    def test_design_docs_anchor_on_cwd_not_the_file(self):
        """SDD specs/plans live under AIRSSTACK_HOME, outside every repo.

        Anchoring them on their own directory can never find a marker, which
        would silently kill design-phase enforcement (e2e case 4).
        """
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            repo = os.path.join(tmp, "repo")
            sdd = os.path.join(tmp, "home", "cc", "plugins", "sdd", "proj", "specs")
            os.makedirs(repo)
            os.makedirs(sdd)
            open(os.path.join(repo, "Cargo.toml"), "w").close()
            spec = os.path.join(sdd, "2026-01-01-x.md")
            self.assertFalse(enforce.marker_active(spec, ["Cargo.toml"], cwd=repo))
            self.assertTrue(enforce.marker_active_in(repo, ["Cargo.toml"]))


class TestRecordSelection(unittest.TestCase):
    def test_prefers_the_record_bound_to_this_project(self):
        records = [
            {"scope": "user", "installPath": "/cache/user"},
            {"scope": "project", "projectPath": "/repo/a", "installPath": "/cache/a"},
        ]
        self.assertEqual(
            enforce.select_record(records, "keyA", {"/repo/a": "keyA"}),
            records[1],
        )

    def test_falls_back_to_the_user_scope_record(self):
        records = [
            {"scope": "project", "projectPath": "/repo/b", "installPath": "/cache/b"},
            {"scope": "user", "installPath": "/cache/user"},
        ]
        self.assertEqual(
            enforce.select_record(records, "keyA", {"/repo/b": "keyB"}),
            records[1],
        )

    def test_project_bound_elsewhere_selects_nothing(self):
        """Anti-leak: installed for repo B only, so it contributes nothing in repo A."""
        records = [
            {"scope": "project", "projectPath": "/repo/b", "installPath": "/cache/b"},
        ]
        self.assertIsNone(
            enforce.select_record(records, "keyA", {"/repo/b": "keyB"})
        )

    def test_local_scope_is_treated_as_project_bound(self):
        records = [
            {"scope": "local", "projectPath": "/repo/a", "installPath": "/cache/a"},
        ]
        self.assertEqual(
            enforce.select_record(records, "keyA", {"/repo/a": "keyA"}),
            records[0],
        )

    def test_scopeless_record_with_no_projectPath_acts_as_user(self):
        records = [{"installPath": "/cache/legacy"}]
        self.assertEqual(enforce.select_record(records, "keyA", {}), records[0])


class TestReadRegistry(unittest.TestCase):
    def test_keeps_only_airsstack_marketplace_keys(self):
        import json as _json
        import tempfile
        payload = {
            "plugins": {
                "airsstack-guideline-rust@airsstack": [{"installPath": "/a"}],
                "superpowers@claude-plugins-official": [{"installPath": "/b"}],
            }
        }
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as fh:
            _json.dump(payload, fh)
            path = fh.name
        try:
            got = enforce.read_registry(path)
            self.assertEqual(list(got.keys()), ["airsstack-guideline-rust@airsstack"])
        finally:
            os.unlink(path)

    def test_unreadable_registry_returns_empty(self):
        self.assertEqual(enforce.read_registry("/nonexistent/registry.json"), {})


class TestSentinels(unittest.TestCase):
    def setUp(self):
        import tempfile
        self.tmp = tempfile.mkdtemp()
        self.old = os.environ.get("TMPDIR")
        os.environ["TMPDIR"] = self.tmp

    def tearDown(self):
        import shutil
        if self.old is None:
            os.environ.pop("TMPDIR", None)
        else:
            os.environ["TMPDIR"] = self.old
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_first_claim_wins_second_loses(self):
        path = enforce.sentinel_path("s1", "main", "rust", "code")
        self.assertTrue(enforce.claim(path))
        self.assertFalse(enforce.claim(path))

    def test_subagent_gets_its_own_shot(self):
        main_path = enforce.sentinel_path("s1", "main", "rust", "code")
        sub_path = enforce.sentinel_path("s1", "agent-7", "rust", "code")
        self.assertNotEqual(main_path, sub_path)
        self.assertTrue(enforce.claim(main_path))
        self.assertTrue(enforce.claim(sub_path))

    def test_components_are_sanitized(self):
        path = enforce.sentinel_path("a/b c", "main", "ru st", "code")
        self.assertNotIn("/", os.path.basename(path))
        self.assertNotIn(" ", os.path.basename(path))

    def test_exactly_one_of_n_concurrent_claims_succeeds(self):
        import threading
        path = enforce.sentinel_path("race", "main", "rust", "code")
        results, lock = [], threading.Lock()
        start = threading.Event()

        def worker():
            start.wait()
            got = enforce.claim(path)
            with lock:
                results.append(got)

        threads = [threading.Thread(target=worker) for _ in range(16)]
        for t in threads:
            t.start()
        start.set()
        for t in threads:
            t.join()
        self.assertEqual(results.count(True), 1, results)

    def test_prune_removes_only_old_airsstack_sentinels(self):
        import time as _time
        old = enforce.sentinel_path("old", "main", "rust", "code")
        fresh = enforce.sentinel_path("fresh", "main", "rust", "code")
        foreign = os.path.join(self.tmp, "unrelated-file")
        for p in (old, fresh, foreign):
            open(p, "w").close()
        stale = _time.time() - (enforce.SENTINEL_MAX_AGE + 60)
        os.utime(old, (stale, stale))
        os.utime(foreign, (stale, stale))
        enforce.prune_sentinels()
        self.assertFalse(os.path.exists(old))
        self.assertTrue(os.path.exists(fresh))
        self.assertTrue(os.path.exists(foreign))

    def test_clear_session_unlinks_that_session_only(self):
        mine_a = enforce.sentinel_path("s1", "main", "rust", "code")
        mine_b = enforce.sentinel_path("s1", "agent-7", "rust", "design")
        theirs = enforce.sentinel_path("s2", "main", "rust", "code")
        for p in (mine_a, mine_b, theirs):
            open(p, "w").close()
        self.assertEqual(enforce.clear_session("s1"), 2)
        self.assertFalse(os.path.exists(mine_a))
        self.assertFalse(os.path.exists(mine_b))
        self.assertTrue(os.path.exists(theirs))

    def test_held_sentinels_excludes_foreign_session_and_agent(self):
        """A foreign session's and a foreign agent's sentinels must never be
        attributed to this (session, agent) context — over-attribution is
        the failure mode that makes the doctor lie about who claimed what."""
        session, agent = "s1", "doctor"
        open(enforce.sentinel_path(session, agent, "rust", "code"), "w").close()
        open(enforce.sentinel_path("other-session", agent, "rust", "code"), "w").close()
        open(enforce.sentinel_path(session, "explorer", "rust", "code"), "w").close()
        self.assertEqual(enforce._held_sentinels(session, agent), ["rust:code"])

    def test_held_sentinels_are_sorted(self):
        """The directory itself may already return entries in sorted order
        (observed on APFS), which would let a dropped `sorted()` call hide
        behind filesystem happenstance. Mock `os.listdir` to hand back a
        deliberately unsorted order so the assertion pins the function's
        own ordering guarantee, not the filesystem's."""
        import unittest.mock as mock
        session, agent = "s1", "doctor"
        unsorted_names = [
            "airsstack-enforce-s1-doctor-zulu-code",
            "airsstack-enforce-s1-doctor-alpha-code",
        ]
        with mock.patch.object(enforce.os, "listdir", return_value=unsorted_names):
            held = enforce._held_sentinels(session, agent)
        self.assertEqual(held, ["alpha:code", "zulu:code"])

    def test_held_sentinels_survives_a_missing_sentinel_dir(self):
        os.environ["TMPDIR"] = os.path.join(self.tmp, "does-not-exist")
        self.assertEqual(enforce._held_sentinels("s1", "doctor"), [])


class TestExplainStages(unittest.TestCase):
    """Each of the framework's silent-exit paths must be named, not merely silent."""

    def setUp(self):
        import tempfile
        self.tmp = tempfile.mkdtemp()
        os.environ["TMPDIR"] = os.path.join(self.tmp, "sentinels")
        os.makedirs(os.environ["TMPDIR"])
        # A plugin dir holding a rust manifest.
        self.plugin = os.path.join(self.tmp, "plugin")
        os.makedirs(self.plugin)
        with open(os.path.join(self.plugin, "enforcement.json"), "w") as fh:
            fh.write(
                '{"stack":"rust","detect":["Cargo.toml"],'
                '"match":["**/*.rs","**/Cargo.toml"],'
                '"skill":"airsstack-guideline-rust:rust-guidelines",'
                '"phase":["code","design"]}'
            )
        # A repo with a Cargo.toml marker.
        self.repo = os.path.join(self.tmp, "repo")
        os.makedirs(os.path.join(self.repo, "src"))
        open(os.path.join(self.repo, "Cargo.toml"), "w").close()
        open(os.path.join(self.repo, "src", "lib.rs"), "w").close()
        # path_for_matching() only reports a repo-relative match path inside
        # a real git worktree; outside one it falls back to the basename.
        # A real repo is required for the "match path: src/lib.rs" trace line.
        import subprocess
        subprocess.check_call(
            ["git", "init", "-q", self.repo],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        self.registry = os.path.join(self.tmp, "registry.json")
        self._registry([{"scope": "user", "installPath": self.plugin}])

    def tearDown(self):
        import shutil
        os.environ.pop("TMPDIR", None)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _registry(self, records):
        import json as _json
        with open(self.registry, "w") as fh:
            _json.dump({"plugins": {"airsstack-guideline-rust@airsstack": records}}, fh)

    def _explain(self, rel="src/lib.rs", registry=None):
        return enforce.explain(
            os.path.join(self.repo, rel),
            self.repo,
            registry=registry if registry is not None else self.registry,
        )

    def test_success_path_names_the_emitted_skill(self):
        text = self._explain()
        self.assertIn("EMIT", text)
        self.assertIn("airsstack-guideline-rust:rust-guidelines", text)
        self.assertIn("outcome: 1 pointer", text)
        # Pin explain()'s own pointer-rendering loop, not resolve()'s EMIT
        # trace line — both happen to contain the skill id, but only the
        # loop renders the "  -> " prefixed, "MANDATORY"-worded pointer text.
        self.assertIn("\n  -> ", text)
        self.assertIn("MANDATORY", text)

    def test_header_reports_path_cwd_and_registry(self):
        """Pins the header FIELDS and their VALUES, not column alignment —
        T2 re-pads these lines for new keys ("python:", "sdd root:", ...);
        the padding is allowed to change, a dropped field or wrong value
        is not."""
        import re
        text = self._explain()
        target = os.path.join(self.repo, "src", "lib.rs")
        self.assertRegex(
            text,
            r"(?m)^path:[ \t]+" + re.escape(os.path.abspath(target)) + r"$",
        )
        self.assertRegex(
            text,
            r"(?m)^cwd:[ \t]+" + re.escape(os.path.abspath(self.repo)) + r"$",
        )
        self.assertRegex(
            text,
            r"(?m)^registry:[ \t]+" + re.escape(self.registry) + r"$",
        )

    def test_doctor_uses_a_dedicated_sentinel_namespace(self):
        """explain()'s `agent='doctor'` default must not collide with the
        hook's own 'main' agent sentinel, or a --explain run would silently
        consume the main thread's one shot at the pointer."""
        self._explain()  # claims the doctor's sentinel for session "enforce-doctor"
        pointers = enforce.resolve(
            os.path.join(self.repo, "src", "lib.rs"), self.repo,
            "enforce-doctor", "main", registry=self.registry,
        )
        self.assertEqual(len(pointers), 1)

    def test_design_doc_phase_is_explained(self):
        home = os.path.join(self.tmp, "home")
        specdir = os.path.join(home, "cc", "plugins", "sdd", "proj", "specs")
        os.makedirs(specdir)
        spec = os.path.join(specdir, "2026-01-01-x.md")
        open(spec, "w").close()
        text = enforce.explain(spec, self.repo, registry=self.registry, home=home)
        self.assertIn("phase: design", text)
        self.assertIn("sdd root: " + enforce.sdd_root(home), text)

    def test_unreadable_registry_is_named(self):
        text = self._explain(registry=os.path.join(self.tmp, "absent.json"))
        self.assertIn("no @airsstack plugins in the registry", text)

    def test_zero_manifests_is_named_with_the_delivery_hint(self):
        os.unlink(os.path.join(self.plugin, "enforcement.json"))
        text = self._explain()
        self.assertIn("zero manifests loaded", text)
        self.assertIn("parity", text.lower())

    def test_gate1_unbound_record_is_named(self):
        self._registry([
            {"scope": "project", "projectPath": os.path.join(self.tmp, "elsewhere"),
             "installPath": self.plugin}
        ])
        text = self._explain()
        self.assertIn("GATE 1", text)

    def test_gate2_missing_marker_is_named(self):
        os.unlink(os.path.join(self.repo, "Cargo.toml"))
        text = self._explain()
        self.assertIn("GATE 2", text)

    def test_gate3_no_glob_hit_is_named(self):
        open(os.path.join(self.repo, "README.md"), "w").close()
        text = self._explain(rel="README.md")
        self.assertIn("GATE 3", text)

    def test_already_claimed_is_named(self):
        self._explain()  # first call claims the sentinel
        text = self._explain()
        self.assertIn("already claimed", text)

    def test_trace_reports_the_project_key_and_match_path(self):
        text = self._explain()
        self.assertIn("project key:", text)
        self.assertIn("match path: src/lib.rs", text)

    def test_success_path_names_the_selected_install_path(self):
        """Spec §9 requires 'record selection' as an output. Before this,
        the chosen installPath was named only on GATE 1 failure — on a
        successful trace it appeared nowhere, so a reader could not tell
        which of several registry records for a key was actually read.
        This is the first thing you want when repo and cache diverge."""
        text = self._explain()
        self.assertIn(
            "airsstack-guideline-rust@airsstack: using " + self.plugin, text
        )

    def test_reports_the_interpreter_and_sentinel_dir(self):
        text = self._explain()
        self.assertIn("python:   %d.%d.%d" % sys.version_info[:3], text)
        self.assertIn(os.environ["TMPDIR"], text)

    def test_lists_the_sentinels_held_for_this_context(self):
        self._explain()
        text = self._explain()
        self.assertIn("sentinels held: rust:code", text)

    def test_reports_no_sentinels_when_none_are_held(self):
        text = self._explain()
        self.assertIn("sentinels held: none", text)

    def test_explain_appends_the_parity_section_when_in_a_source_repo(self):
        """A 'source repo' is one with a top-level plugins/ dir. self.repo (a
        bare `git init` with no plugins/) is deliberately not one until this
        test adds it — parity_report's own early return treats an absent
        plugins/ as out-of-scope, and explain() must mirror that (see the
        adaptation note in TestParity's own docstring)."""
        os.makedirs(os.path.join(self.repo, "plugins"))
        text = enforce.explain(
            os.path.join(self.repo, "src", "lib.rs"), self.repo, registry=self.registry
        )
        self.assertIn("parity:", text)

    def test_explain_reports_drift_when_repo_and_cache_disagree(self):
        """Distinct from the case above: this exercises the actual `if
        drift:` branch (report lines + the cache-runs-from-here hint), not
        just the empty-drift 'repo and cache agree' fallback."""
        plugin_src = os.path.join(self.repo, "plugins", "airsstack-guideline-rust")
        os.makedirs(os.path.join(plugin_src, ".claude-plugin"))
        manifest_json = '{"name":"airsstack-guideline-rust","version":"0.1.0"}'
        with open(os.path.join(plugin_src, ".claude-plugin", "plugin.json"), "w") as fh:
            fh.write(manifest_json)
        os.makedirs(os.path.join(self.plugin, ".claude-plugin"))
        with open(os.path.join(self.plugin, ".claude-plugin", "plugin.json"), "w") as fh:
            fh.write(manifest_json)
        with open(os.path.join(plugin_src, "extra.txt"), "w") as fh:
            fh.write("present in source only")
        text = self._explain()
        self.assertIn("parity: 1 file(s) out of sync", text)
        self.assertIn("extra.txt", text)
        self.assertIn("MISSING from cache", text)
        self.assertIn("dispatcher runs from the cache", text)

    def test_drift_listing_is_capped_at_twenty_with_a_remainder_note(self):
        """`explain()` renders at most 20 drift lines and names how many
        more exist past the cap — unpinned before this test (mutation
        `drift[:20]` -> `drift[:1]` previously survived the full gate)."""
        plugin_src = os.path.join(self.repo, "plugins", "airsstack-guideline-rust")
        os.makedirs(os.path.join(plugin_src, ".claude-plugin"))
        manifest_json = '{"name":"airsstack-guideline-rust","version":"0.1.0"}'
        with open(os.path.join(plugin_src, ".claude-plugin", "plugin.json"), "w") as fh:
            fh.write(manifest_json)
        os.makedirs(os.path.join(self.plugin, ".claude-plugin"))
        with open(os.path.join(self.plugin, ".claude-plugin", "plugin.json"), "w") as fh:
            fh.write(manifest_json)
        for i in range(25):
            with open(os.path.join(plugin_src, "extra-%02d.txt" % i), "w") as fh:
                fh.write("present in source only")
        text = self._explain()
        self.assertIn("parity: 25 file(s) out of sync", text)
        self.assertIn("(+5 more)", text)
        rendered = [
            line for line in text.splitlines()
            if line.startswith("  airsstack-guideline-rust: extra-")
        ]
        self.assertEqual(len(rendered), 20)

    def test_headline_counts_only_out_of_sync_not_unreadable_entries(self):
        """T4-review carry-over (blue): the headline claims 'N file(s) out of
        sync', but `drift` also carries 'source tree unreadable, parity
        unknown' lines for a plugin whose source could not even be compared
        -- counting those into N asserts precisely what the very next detail
        line says is unknown. Register two plugin keys: one with a genuine
        MISSING-from-cache drift, and a second whose source directory is
        wholly unreadable. The headline must count only the first."""
        import json as _json

        plugin_a_src = os.path.join(self.repo, "plugins", "airsstack-guideline-rust")
        os.makedirs(os.path.join(plugin_a_src, ".claude-plugin"))
        manifest_json = '{"name":"airsstack-guideline-rust","version":"0.1.0"}'
        with open(os.path.join(plugin_a_src, ".claude-plugin", "plugin.json"), "w") as fh:
            fh.write(manifest_json)
        os.makedirs(os.path.join(self.plugin, ".claude-plugin"))
        with open(os.path.join(self.plugin, ".claude-plugin", "plugin.json"), "w") as fh:
            fh.write(manifest_json)
        with open(os.path.join(plugin_a_src, "extra.txt"), "w") as fh:
            fh.write("present in source only")

        plugin_b_src = os.path.join(self.repo, "plugins", "airsstack-journal")
        os.makedirs(plugin_b_src)
        plugin_b_cache = os.path.join(self.tmp, "cache-b")
        os.makedirs(plugin_b_cache)
        with open(self.registry, "w") as fh:
            _json.dump({
                "plugins": {
                    "airsstack-guideline-rust@airsstack": [
                        {"scope": "user", "installPath": self.plugin}
                    ],
                    "airsstack-journal@airsstack": [
                        {"scope": "user", "installPath": plugin_b_cache}
                    ],
                }
            }, fh)

        os.chmod(plugin_b_src, 0o000)
        try:
            text = self._explain()
        finally:
            os.chmod(plugin_b_src, 0o755)

        self.assertIn("parity: 1 file(s) out of sync between repo and cache", text)
        self.assertIn("unreadable, parity unknown", text)


class TestCli(unittest.TestCase):
    """`_cli` must always return 0 — both `--explain` branches (reviewer's
    M11/M12), and it must survive a broken or failing stdout (D10)."""

    def setUp(self):
        import tempfile
        self.tmp = tempfile.mkdtemp()
        self.old_tmpdir = os.environ.get("TMPDIR")
        self.old_registry = os.environ.get("AIRSSTACK_ENFORCE_REGISTRY")
        os.environ["TMPDIR"] = self.tmp
        os.environ["AIRSSTACK_ENFORCE_REGISTRY"] = os.path.join(self.tmp, "nope.json")

    def tearDown(self):
        import shutil
        if self.old_tmpdir is None:
            os.environ.pop("TMPDIR", None)
        else:
            os.environ["TMPDIR"] = self.old_tmpdir
        if self.old_registry is None:
            os.environ.pop("AIRSSTACK_ENFORCE_REGISTRY", None)
        else:
            os.environ["AIRSSTACK_ENFORCE_REGISTRY"] = self.old_registry
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_explain_branch_with_a_path_returns_zero(self):
        import io
        import contextlib
        target = os.path.join(self.tmp, "lib.rs")
        open(target, "w").close()
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            rc = enforce._cli(["--explain", target])
        self.assertEqual(rc, 0)
        self.assertIn("outcome:", buf.getvalue())

    def test_explain_branch_with_no_path_returns_zero(self):
        import io
        import contextlib
        buf = io.StringIO()
        err = io.StringIO()
        with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
            rc = enforce._cli(["--explain"])
        self.assertEqual(rc, 0)

    def test_survives_a_broken_stdout_pipe(self):
        """Mirrors the reviewer's measured case: --explain into a closed
        pipe exits 120, not because the write itself raises inside _cli,
        but because a small write is buffered and only actually hits the
        OS — and fails — when CPython flushes stdout at interpreter
        shutdown, after _cli has already returned. So it is not enough for
        _cli to guard its own write; a flush performed after _cli returns
        (standing in for that shutdown-time flush) must also not raise."""
        import io
        target = os.path.join(self.tmp, "lib.rs")
        open(target, "w").close()
        r, w = os.pipe()
        os.close(r)  # closed read end: any write raises BrokenPipeError
        fake = os.fdopen(w, "w")
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stdout = fake
        # The failure handler now also redirects stderr; a StringIO's
        # fileno() raises, which the handler's own guard absorbs — this
        # keeps the real test process's stderr out of the blast radius
        # instead of letting the handler dup2 over the live fd.
        sys.stderr = io.StringIO()
        try:
            rc = enforce._cli(["--explain", target])
            self.assertEqual(rc, 0)
            fake.flush()  # stands in for CPython's shutdown-time flush
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr
            try:
                fake.close()
            except OSError:
                pass

    def test_survives_a_generic_stdout_write_error(self):
        """Mirrors the reviewer's second measurement (`> /dev/full`): a
        plain OSError on write, not just BrokenPipeError."""
        import io
        target = os.path.join(self.tmp, "lib.rs")
        open(target, "w").close()
        r, w = os.pipe()
        os.close(r)

        class Full:
            def __init__(self, fd):
                self._fd = fd

            def write(self, _):
                raise OSError(28, "No space left on device")

            def flush(self):
                pass

            def fileno(self):
                return self._fd

        fake = Full(w)
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stdout = fake
        sys.stderr = io.StringIO()  # keep the real stderr fd out of the blast radius
        try:
            rc = enforce._cli(["--explain", target])
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr
            os.close(w)
        self.assertEqual(rc, 0)

    def test_survives_a_broken_stderr_pipe_on_the_usage_branch(self):
        """D10, the no-path branch: the usage message writes to stderr, not
        stdout. A closed stderr must get the same shutdown-time-flush guard
        as stdout (reviewer NEW-2) — otherwise this one branch still exits
        120 while every other branch exits 0."""
        import io
        r, w = os.pipe()
        os.close(r)  # closed read end: any write raises BrokenPipeError
        fake_err = os.fdopen(w, "w")
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stderr = fake_err
        sys.stdout = io.StringIO()  # keep the real stdout fd out of the blast radius
        try:
            rc = enforce._cli(["--explain"])
            self.assertEqual(rc, 0)
            fake_err.flush()  # stands in for CPython's shutdown-time flush
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr
            try:
                fake_err.close()
            except OSError:
                pass

    def test_broken_explain_is_loud_on_stderr_not_silent(self):
        """D10 requires exit 0, not silence: a genuine defect inside
        explain() must still exit 0, but it must say so on stderr rather
        than vanish — the exact indistinguishable-silence class spec §9
        exists to abolish (reviewer NEW-3)."""
        import io
        target = os.path.join(self.tmp, "lib.rs")
        open(target, "w").close()
        old_explain = enforce.explain

        def broken_explain(*args, **kwargs):
            raise NameError("name 'undefined_symbol' is not defined")

        enforce.explain = broken_explain
        buf_out = io.StringIO()
        buf_err = io.StringIO()
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stdout = buf_out
        sys.stderr = buf_err
        try:
            rc = enforce._cli(["--explain", target])
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr
            enforce.explain = old_explain
        self.assertEqual(rc, 0)
        self.assertIn("NameError", buf_err.getvalue())


class TestParity(unittest.TestCase):
    """Reproduce B1: a manifest present in source, missing from the cache."""

    def setUp(self):
        import tempfile
        self.tmp = tempfile.mkdtemp()
        self.top = os.path.join(self.tmp, "repo")
        self.src = os.path.join(self.top, "plugins", "airsstack-guideline-rust")
        os.makedirs(os.path.join(self.src, ".claude-plugin"))
        with open(os.path.join(self.src, ".claude-plugin", "plugin.json"), "w") as fh:
            fh.write('{"name":"airsstack-guideline-rust","version":"0.1.0"}')
        with open(os.path.join(self.src, "enforcement.json"), "w") as fh:
            fh.write('{"stack":"rust"}')
        self.cache = os.path.join(self.tmp, "cache", "airsstack-guideline-rust", "0.1.0")
        os.makedirs(os.path.join(self.cache, ".claude-plugin"))
        with open(os.path.join(self.cache, ".claude-plugin", "plugin.json"), "w") as fh:
            fh.write('{"name":"airsstack-guideline-rust","version":"0.1.0"}')
        self.records = {
            "airsstack-guideline-rust@airsstack": [{"installPath": self.cache}]
        }

    def tearDown(self):
        import shutil
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_missing_cache_file_is_reported(self):
        """Pins the whole line, not two independent substrings — two
        separate `assertIn`s would also pass if 'enforcement.json' and
        'MISSING from cache' happened to appear on two different report
        lines rather than together on the B1 line."""
        report = enforce.parity_report(self.top, self.records)
        self.assertIn(
            "airsstack-guideline-rust: enforcement.json MISSING from cache",
            report,
        )

    def test_differing_file_is_reported(self):
        """Pins `filecmp.cmp(..., shallow=False)`. The cache content is the
        same SIZE as the source's ('{"stack":"java"}' vs '{"stack":"rust"}',
        16 bytes each) and its mtime is forced (via os.utime) to match the
        source's exactly, so `shallow=True` — which only compares os.stat's
        (mode, size, mtime) signature, never file content — cannot tell them
        apart. Only the actual content read `shallow=False` performs catches
        this drift, which is exactly the class of drift (e.g. a version bump
        with no size change) this check exists to catch."""
        src_path = os.path.join(self.src, "enforcement.json")
        dest_path = os.path.join(self.cache, "enforcement.json")
        with open(dest_path, "w") as fh:
            fh.write('{"stack":"java"}')
        st = os.stat(src_path)
        os.utime(dest_path, (st.st_atime, st.st_mtime))
        report = enforce.parity_report(self.top, self.records)
        self.assertIn("DIFFERS", "\n".join(report))

    def test_duplicate_install_path_records_are_not_double_counted(self):
        """Reviewer finding: several registry records for the same key can
        point at the SAME cache dir (this machine commonly has 2-3 per key).
        `parity_report` must de-duplicate `installPath` the way
        `cache_sync.resolve_install_paths` already does, or every file
        reports once per record and the headline count is inflated."""
        records = {
            "airsstack-guideline-rust@airsstack": [
                {"installPath": self.cache},
                {"installPath": self.cache},
                {"installPath": self.cache},
            ]
        }
        report = enforce.parity_report(self.top, records)
        missing = [
            line for line in report
            if "enforcement.json" in line and "MISSING from cache" in line
        ]
        self.assertEqual(len(missing), 1)

    def test_missing_cache_dir_reports_every_source_file_missing(self):
        """Reviewer finding: a registry-listed plugin whose cache dir does
        not exist AT ALL must not fall through to 'repo and cache agree' —
        it is the most complete delivery failure possible, and the section
        whose whole purpose is naming delivery failures must not say
        everything is fine. Every source file must be reported MISSING."""
        import shutil
        shutil.rmtree(self.cache)
        report = enforce.parity_report(self.top, self.records)
        self.assertIn(
            "airsstack-guideline-rust: enforcement.json MISSING from cache",
            report,
        )
        self.assertIn(
            "airsstack-guideline-rust: %s MISSING from cache"
            % os.path.join(".claude-plugin", "plugin.json"),
            report,
        )

    def test_identical_trees_report_nothing(self):
        import shutil
        shutil.copy2(
            os.path.join(self.src, "enforcement.json"),
            os.path.join(self.cache, "enforcement.json"),
        )
        self.assertEqual(enforce.parity_report(self.top, self.records), [])

    def test_ignored_names_are_not_reported(self):
        import shutil
        shutil.copy2(
            os.path.join(self.src, "enforcement.json"),
            os.path.join(self.cache, "enforcement.json"),
        )
        open(os.path.join(self.cache, ".in_use"), "w").close()
        self.assertEqual(enforce.parity_report(self.top, self.records), [])

    def test_outside_a_plugin_source_repo_reports_nothing(self):
        self.assertEqual(enforce.parity_report(self.tmp, self.records), [])

    def test_ignored_names_in_source_are_not_walked(self):
        """The plan's own `test_ignored_names_are_not_reported` plants
        `.in_use` only in the cache, which the walk never enumerates
        (parity_report iterates `_tree_files(src_dir)`, not the cache tree),
        so it never actually exercises the filename-level filter in
        `_tree_files`. This closes that gap directly with a top-level
        ignored FILE (not a nested ignored directory, which the separate
        `dirnames[:]` pruning line would catch on its own): an ignored name
        that exists in source but not in cache must not be reported as
        MISSING."""
        import shutil
        open(os.path.join(self.src, ".DS_Store"), "w").close()
        shutil.copy2(
            os.path.join(self.src, "enforcement.json"),
            os.path.join(self.cache, "enforcement.json"),
        )
        self.assertEqual(enforce.parity_report(self.top, self.records), [])

    def test_ignored_directories_in_source_are_pruned(self):
        """A file nested under an ignored directory name (e.g. a stray
        `.git`) must not surface as MISSING from cache — this pins the
        `dirnames[:]` pruning line in `_tree_files`, distinct from the
        filename-level filter pinned above."""
        import shutil
        os.makedirs(os.path.join(self.src, ".git"))
        with open(os.path.join(self.src, ".git", "HEAD"), "w") as fh:
            fh.write("ref: refs/heads/main")
        shutil.copy2(
            os.path.join(self.src, "enforcement.json"),
            os.path.join(self.cache, "enforcement.json"),
        )
        self.assertEqual(enforce.parity_report(self.top, self.records), [])

    def test_key_with_no_source_manifest_is_skipped(self):
        """A registry key with no `.claude-plugin/plugin.json` under
        `plugins/<name>/` (e.g. a stray or unrelated key) must be skipped
        outright, not walked or reported against — pins the manifest-exists
        guard distinct from the directory-exists guard above. The orphan's
        source dir is populated (not merely absent) so the guard's removal
        is actually observable: an absent dir yields no files to walk
        either way, which would let the mutation hide undetected."""
        import shutil
        shutil.copy2(
            os.path.join(self.src, "enforcement.json"),
            os.path.join(self.cache, "enforcement.json"),
        )
        orphan_src = os.path.join(self.top, "plugins", "orphan-plugin")
        os.makedirs(orphan_src)
        with open(os.path.join(orphan_src, "notes.txt"), "w") as fh:
            fh.write("not a real plugin source")
        orphan_cache = os.path.join(self.tmp, "cache", "orphan-plugin", "0.1.0")
        os.makedirs(orphan_cache)
        records = dict(self.records)
        records["orphan-plugin@airsstack"] = [{"installPath": orphan_cache}]
        self.assertEqual(enforce.parity_report(self.top, records), [])

    def test_unreadable_source_dir_is_reported_not_silently_agreed(self):
        """T3-review carry-over (blue): an unreadable source dir must not
        fall through to 'repo and cache agree' — that is the same false
        all-clear the missing-cache-dir fix closed, just from the source
        side instead of the cache side. Both chmod modes are checked
        because a fix that only covers one of them closes a narrower case
        than the one measured against B1's parity check:

        - `0o111` (execute-only, no read) still lets `os.path.isfile` stat
          the nested plugin.json directly (traversal needs only +x on
          ancestors), so it must be `os.walk`'s `os.scandir(src_dir)` — which
          needs +r — that catches it.
        - `0o000` (no bits at all) fails traversal itself, so
          `os.path.isfile` on the nested plugin.json raises internally —
          `isfile()` swallows every OSError and reports False identically
          whether the manifest is genuinely absent or merely unreachable.
          A guard that trusts `isfile() is False` to mean "no manifest
          here" `continue`s before `os.walk` is ever reached, and the
          directory is silently treated as agreeing with the cache."""
        for mode in (0o111, 0o000):
            with self.subTest(mode=oct(mode)):
                os.chmod(self.src, mode)
                try:
                    report = enforce.parity_report(self.top, self.records)
                finally:
                    os.chmod(self.src, 0o755)
                self.assertTrue(
                    any("unreadable" in line for line in report),
                    "mode %s: expected an 'unreadable' line, got: %r"
                    % (oct(mode), report),
                )
                self.assertFalse(
                    any("MISSING" in line for line in report),
                    "mode %s: an unreadable source tree must not be reported as "
                    "files MISSING (that implies the source WAS read and compared)"
                    % (oct(mode),),
                )

    def test_key_with_absent_source_dir_is_skipped(self):
        """T4-review carry-over (blue): a registry key with NO source
        directory at all under `plugins/<name>/` (a plugin simply not
        developed in this repo) must be skipped outright -- distinct from
        `test_key_with_no_source_manifest_is_skipped` above, whose orphan
        directory is populated and therefore stops one guard later (the
        manifest-exists check). This exercises the earlier `if not
        os.path.isdir(src_dir): continue` guard on its own: without it,
        `os.listdir` on a nonexistent path raises, and a plugin that was
        never checked out here would falsely report 'source tree
        unreadable, parity unknown'."""
        import shutil
        shutil.copy2(
            os.path.join(self.src, "enforcement.json"),
            os.path.join(self.cache, "enforcement.json"),
        )
        absent_cache = os.path.join(self.tmp, "cache", "absent-plugin", "0.1.0")
        os.makedirs(absent_cache)
        records = dict(self.records)
        records["absent-plugin@airsstack"] = [{"installPath": absent_cache}]
        # Deliberately no plugins/absent-plugin/ directory in source at all.
        self.assertEqual(enforce.parity_report(self.top, records), [])

    def test_unreadable_nested_subdir_is_reported_not_silently_agreed(self):
        """T4-review carry-over (blue): `os.walk`'s default `onerror=None`
        swallows a `PermissionError` raised while listing a NESTED
        subdirectory and simply yields fewer files, so an unreadable
        subdirectory one level below `src_dir` reads back identically to an
        empty one -- the same false all-clear the top-level `os.listdir`
        probe (`test_unreadable_source_dir_is_reported_not_silently_agreed`
        above) closes, just one level deeper. `src_dir` itself stays fully
        readable here so that top-level probe passes clean; only the nested
        subdirectory is unreadable, which only `_tree_files`' `onerror`
        argument and the `if walk_errors:` check that follows it can catch."""
        import shutil
        shutil.copy2(
            os.path.join(self.src, "enforcement.json"),
            os.path.join(self.cache, "enforcement.json"),
        )
        nested = os.path.join(self.src, "nested")
        os.makedirs(nested)
        with open(os.path.join(nested, "inner.txt"), "w") as fh:
            fh.write("only reachable if permissions allow it")
        os.chmod(nested, 0o000)
        try:
            report = enforce.parity_report(self.top, self.records)
        finally:
            os.chmod(nested, 0o755)
        self.assertTrue(
            any("unreadable" in line for line in report),
            "expected an 'unreadable' line for the nested subdir, got: %r" % (report,),
        )
        self.assertFalse(
            any("MISSING" in line for line in report),
            "an unreadable nested subdir must not be reported as files "
            "MISSING (that implies the source subtree WAS read and compared)",
        )

    def test_tree_files_are_sorted(self):
        """Mirrors `test_held_sentinels_are_sorted`: the filesystem may
        already hand back sorted names (observed on APFS), which would let
        a dropped `sorted()` in `_tree_files` hide behind that happenstance.
        Mock `os.walk` to hand back a deliberately unsorted order so the
        assertion pins the function's own ordering guarantee, not the
        filesystem's."""
        import unittest.mock as mock
        walk_result = iter([(self.src, [], ["zulu.txt", "alpha.txt", "mid.txt"])])
        with mock.patch.object(enforce.os, "walk", return_value=walk_result):
            found = enforce._tree_files(self.src)
        self.assertEqual(found, ["alpha.txt", "mid.txt", "zulu.txt"])


if __name__ == "__main__":
    unittest.main()
