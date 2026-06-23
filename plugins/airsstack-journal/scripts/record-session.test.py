#!/usr/bin/env python3
"""Black-box tests for record-session.py — run against a temp vault."""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
RECORDER = HERE / "record-session.py"

FACTS_BEGIN = "<!-- journal:facts:begin -->"
FACTS_END = "<!-- journal:facts:end -->"
NARR_BEGIN = "<!-- journal:narrative:begin -->"
NARR_END = "<!-- journal:narrative:end -->"

TRANSCRIPT_LINES = [
    '{"type":"user","gitBranch":"main","message":{"content":"implement the parser"}}',
    '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test --all-features"}}]}}',
    '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/work/src/lib.rs"}}]}}',
]


class RecordSessionTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.home = Path(self.tmp)
        self.vault = self.home / "journal"
        for d in ("daily", "sessions", "notes", "mocs", ".index"):
            (self.vault / d).mkdir(parents=True, exist_ok=True)
        self.transcript = self.home / "t.jsonl"
        self.transcript.write_text("\n".join(TRANSCRIPT_LINES) + "\n", encoding="utf-8")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_recorder(self, session_id="abc12345-dead-beef", transcript=None, cwd=""):
        env = dict(os.environ, AIRSSTACK_HOME=str(self.home))
        tpath = str(self.transcript if transcript is None else transcript)
        return subprocess.run(
            [sys.executable, str(RECORDER), session_id, tpath, cwd],
            env=env, capture_output=True, text=True)

    def note(self):
        return (self.vault / "sessions" / "session-abc12345.md").read_text(encoding="utf-8")

    def region(self, text, begin, end):
        i = text.find(begin)
        j = text.find(end, i + len(begin)) if i != -1 else -1
        return text[i + len(begin):j] if (i != -1 and j != -1) else None

    def test_creates_session_note_with_both_regions(self):
        res = self.run_recorder()
        self.assertEqual(res.returncode, 0, res.stderr)
        text = self.note()
        self.assertIn("type: session", text)
        self.assertIsNotNone(self.region(text, FACTS_BEGIN, FACTS_END))
        self.assertIsNotNone(self.region(text, NARR_BEGIN, NARR_END))

    def test_facts_capture_branch_files_commands_intent(self):
        self.run_recorder()
        facts = self.region(self.note(), FACTS_BEGIN, FACTS_END)
        self.assertIn("branch: main", facts)
        self.assertIn("implement the parser", facts)
        self.assertIn("/work/src/lib.rs", facts)
        self.assertIn("cargo test --all-features", facts)

    def test_vault_writes_are_recorded_as_notes_not_files(self):
        lines = TRANSCRIPT_LINES + [
            '{"type":"assistant","message":{"content":[{"type":"tool_use",'
            '"name":"Write","input":{"file_path":"%s/notes/tokio-cancel.md"}}]}}'
            % str(self.vault),
        ]
        self.transcript.write_text("\n".join(lines) + "\n", encoding="utf-8")
        self.run_recorder()
        facts = self.region(self.note(), FACTS_BEGIN, FACTS_END)
        self.assertIn("[[tokio-cancel]]", facts)

    def test_preserves_existing_narrative_region(self):
        note_path = self.vault / "sessions" / "session-abc12345.md"
        note_path.write_text(
            "---\ntitle: t\ntype: session\nsummary: distilled\n---\n"
            + FACTS_BEGIN + "\n\n" + FACTS_END + "\n"
            + NARR_BEGIN + "\nMY NARRATIVE [[x]]\n" + NARR_END + "\n",
            encoding="utf-8")
        self.run_recorder()
        text = self.note()
        self.assertIn("MY NARRATIVE [[x]]", text)
        self.assertIn("summary: distilled", text)
        self.assertIn("branch: main", self.region(text, FACTS_BEGIN, FACTS_END))

    def test_facts_region_is_idempotent(self):
        self.run_recorder()
        first = self.region(self.note(), FACTS_BEGIN, FACTS_END)
        self.run_recorder()
        second = self.region(self.note(), FACTS_BEGIN, FACTS_END)
        self.assertEqual(first, second)

    def test_links_session_into_daily_note(self):
        self.run_recorder()
        dailies = list((self.vault / "daily").glob("*.md"))
        self.assertEqual(len(dailies), 1)
        self.assertIn("[[session-abc12345]]", dailies[0].read_text(encoding="utf-8"))

    def test_refreshes_index(self):
        self.run_recorder()
        self.assertTrue((self.vault / ".index" / "summaries.tsv").exists())

    def test_malformed_lines_skipped_not_fatal(self):
        self.transcript.write_text(
            "not json\n" + "\n".join(TRANSCRIPT_LINES) + "\n", encoding="utf-8")
        res = self.run_recorder()
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertIn("branch: main", self.region(self.note(), FACTS_BEGIN, FACTS_END))


if __name__ == "__main__":
    unittest.main()
