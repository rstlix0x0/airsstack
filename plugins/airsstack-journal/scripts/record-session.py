#!/usr/bin/env python3
"""airsstack-journal SessionEnd recorder (deterministic, no model).

Parse the session transcript JSONL for facts and write them into the facts
region of the session note `sessions/session-<id8>.md`, preserving any
existing narrative region. Then link the session into the daily note and
refresh the recall index by invoking the Phase-1 builder. The calling wrapper
enforces fail-open; this worker assumes python3 is present when run but
tolerates a missing or partial transcript.

Usage: record-session.py <session_id> <transcript_path> <cwd>
"""
import importlib.util
import json
import os
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent

FACTS_BEGIN = "<!-- journal:facts:begin -->"
FACTS_END = "<!-- journal:facts:end -->"
NARR_BEGIN = "<!-- journal:narrative:begin -->"
NARR_END = "<!-- journal:narrative:end -->"

EDIT_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit"}
FILE_KEYS = ("file_path", "notebook_path")


def _load_parse_frontmatter():
    spec = importlib.util.spec_from_file_location(
        "journal_build_index", str(HERE / "build-index.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.parse_frontmatter


def vault_root() -> Path:
    home = os.environ.get("AIRSSTACK_HOME") or os.path.expanduser("~/.airsstack")
    return Path(home) / "journal"


def project_key(cwd: str) -> str:
    try:
        res = subprocess.run(
            ["sh", str(HERE / "project-key.sh")],
            cwd=cwd or None, capture_output=True, text=True, timeout=10)
        return res.stdout.strip() or "unknown"
    except Exception:
        return "unknown"


def _truncate(text, limit=200) -> str:
    return " ".join(str(text).split())[:limit]


def _dedup(items):
    seen = []
    for it in items:
        if it and it not in seen:
            seen.append(it)
    return seen


def _first_text(content) -> str:
    if isinstance(content, str):
        return _truncate(content)
    if isinstance(content, list):
        for block in content:
            if isinstance(block, str):
                return _truncate(block)
            if isinstance(block, dict) and block.get("type") == "text":
                return _truncate(block.get("text", ""))
    return ""


def iter_transcript(path: Path):
    try:
        with path.open(encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    yield json.loads(line)
                except ValueError:
                    continue
    except OSError:
        return


def extract_facts(records, root: Path) -> dict:
    branch = ""
    files, notes, commands, intent = [], [], [], ""
    note_prefixes = (str(root / "notes") + os.sep,
                     str(root / "sessions") + os.sep)
    for rec in records:
        if not branch and rec.get("gitBranch"):
            branch = str(rec["gitBranch"])
        msg = rec.get("message") or {}
        content = msg.get("content")
        if rec.get("type") == "user" and not intent:
            intent = _first_text(content)
        if not isinstance(content, list):
            continue
        for block in content:
            if not isinstance(block, dict) or block.get("type") != "tool_use":
                continue
            name = block.get("name", "")
            inp = block.get("input") or {}
            if name == "Bash":
                cmd = str(inp.get("command", "")).strip()
                if cmd:
                    commands.append(cmd.splitlines()[0])
            elif name in EDIT_TOOLS:
                for key in FILE_KEYS:
                    path = inp.get(key)
                    if not path:
                        continue
                    path = str(path)
                    if path.startswith(note_prefixes):
                        notes.append(Path(path).stem.lower())
                    else:
                        files.append(path)
    return {
        "branch": branch,
        "files": _dedup(files),
        "notes": _dedup(notes),
        "commands": _dedup(commands),
        "intent": intent,
    }


def render_facts(facts: dict) -> str:
    lines = [
        "- branch: %s" % (facts["branch"] or "(unknown)"),
        "- intent: %s" % (facts["intent"] or "(none captured)"),
    ]
    if facts["files"]:
        lines.append("- files:")
        lines.extend("  - %s" % f for f in facts["files"])
    if facts["notes"]:
        lines.append("- notes:")
        lines.extend("  - [[%s]]" % n for n in facts["notes"])
    if facts["commands"]:
        lines.append("- commands:")
        lines.extend("  - `%s`" % c for c in facts["commands"])
    return "\n".join(lines)


def _between(text, begin, end):
    i = text.find(begin)
    if i == -1:
        return None
    j = text.find(end, i + len(begin))
    if j == -1:
        return None
    return text[i + len(begin):j]


def _scalar(val) -> str:
    if isinstance(val, list):
        return ", ".join(str(x) for x in val)
    return str(val)


def build_note(existing: str, facts: dict, project: str, now: str) -> str:
    fm, narrative = {}, ""
    if existing:
        try:
            fm, body = _load_parse_frontmatter()(existing)
        except ValueError:
            fm, body = {}, ""
        got = _between(body, NARR_BEGIN, NARR_END)
        if got is not None:
            narrative = got.strip("\n")
    created = fm.get("created") or now
    title = fm.get("title") or ("Session %s" % now)
    summary = fm.get("summary") or ""
    helped = fm.get("helped") or "0"

    front = "\n".join([
        "---",
        "title: %s" % _scalar(title),
        "type: session",
        "project: %s" % _scalar(project),
        "created: %s" % _scalar(created),
        "updated: %s" % now,
        "summary: %s" % _scalar(summary),
        "helped: %s" % _scalar(helped),
        "---",
    ])
    facts_block = "%s\n%s\n%s" % (FACTS_BEGIN, render_facts(facts), FACTS_END)
    narr_block = "%s\n%s\n%s" % (NARR_BEGIN, narrative, NARR_END)
    return front + "\n" + facts_block + "\n" + narr_block + "\n"


def main(argv) -> int:
    session_id = argv[0] if len(argv) > 0 else ""
    transcript = argv[1] if len(argv) > 1 else ""
    cwd = argv[2] if len(argv) > 2 else ""
    id8 = re.sub(r"[^A-Za-z0-9]", "", session_id)[:8].lower()
    if not id8:
        return 0
    stem = "session-%s" % id8

    root = vault_root()
    (root / "sessions").mkdir(parents=True, exist_ok=True)
    note_path = root / "sessions" / ("%s.md" % stem)

    records = list(iter_transcript(Path(transcript))) if transcript else []
    facts = extract_facts(records, root)
    project = project_key(cwd)
    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    today = datetime.now().strftime("%Y-%m-%d")

    existing = note_path.read_text(encoding="utf-8") if note_path.exists() else ""
    note_path.write_text(build_note(existing, facts, project, now), encoding="utf-8")

    subprocess.run(["sh", str(HERE / "daily-link.sh"), today, stem],
                   capture_output=True)
    subprocess.run([sys.executable, str(HERE / "build-index.py")],
                   capture_output=True)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
