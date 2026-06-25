#!/usr/bin/env python3
"""Build the airsstack-journal derived recall index from the Markdown corpus.

Scans daily/, sessions/, notes/, mocs/ under the vault and writes
.index/graph.json, .index/tags.json, .index/summaries.tsv, and the enriched
.index/index.json (nodes + structurally-typed edges + backlinks + unresolved)
consumed by the recall subagent.

Fail-open: a malformed note is skipped with a stderr diagnostic; the rest still
index. The Markdown corpus is the sole source of truth and the output is fully
reconstructible from it. A `--force` flag is accepted; the builder always
performs a full rebuild, so the flag is a marker of intent, not a mode switch.
"""
import json
import os
import re
import sys
from pathlib import Path

NOTE_DIRS = ("daily", "sessions", "notes", "mocs")
WIKILINK_RE = re.compile(r"\[\[([^\]]+)\]\]")
FENCED_CODE_RE = re.compile(r"(?s)(```|~~~).*?\1")
INLINE_CODE_RE = re.compile(r"`[^`\n]*`")
UNRESOLVED_KEY = "_unresolved"
CONTAINER_TYPES = ("session", "daily")
EDGE_PRIORITY = {"supersedes": 4, "depends-on": 3, "contains": 2, "references": 1}


def vault_root() -> Path:
    home = os.environ.get("AIRSSTACK_HOME") or os.path.expanduser("~/.airsstack")
    return Path(home) / "journal"


def parse_value(val: str):
    val = val.strip()
    if val.startswith("[") and val.endswith("]"):
        inner = val[1:-1].strip()
        if not inner:
            return []
        return [item.strip().strip('"').strip("'") for item in inner.split(",")]
    return val.strip('"').strip("'")


def parse_frontmatter(text: str):
    """Return (frontmatter_dict, body). Raise ValueError on malformed frontmatter.

    Supports a leading '---' fence of flat 'key: value' pairs, where a value is
    a scalar or an inline '[a, b, c]' list. A note without a fence yields
    ({}, text).
    """
    if not text.startswith("---"):
        return {}, text
    lines = text.splitlines()
    end = None
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            end = i
            break
    if end is None:
        raise ValueError("unterminated frontmatter fence")
    frontmatter = {}
    for raw in lines[1:end]:
        if not raw.strip():
            continue
        if ":" not in raw:
            raise ValueError("frontmatter line without ':' -> %r" % raw)
        key, _, val = raw.partition(":")
        frontmatter[key.strip()] = parse_value(val)
    body = "\n".join(lines[end + 1:])
    return frontmatter, body


def as_list(val):
    if val is None:
        return []
    if isinstance(val, list):
        return [str(x) for x in val]
    return [str(val)]


def scalar(val) -> str:
    if isinstance(val, list):
        return ", ".join(str(x) for x in val)
    return str(val)


def stem_of(path: Path) -> str:
    return path.stem.lower()


def node_record(path: Path, frontmatter, root: Path):
    try:
        helped = int(scalar(frontmatter.get("helped", "0")) or "0")
    except ValueError:
        helped = 0
    return {
        "type": scalar(frontmatter.get("type", "")),
        "title": scalar(frontmatter.get("title", "")),
        "summary": scalar(frontmatter.get("summary", "")),
        "project": scalar(frontmatter.get("project", "")),
        "domains": [d.strip() for d in as_list(frontmatter.get("domains")) if d.strip()],
        "tags": [t.strip() for t in as_list(frontmatter.get("tags")) if t.strip()],
        "helped": helped,
        "updated": scalar(frontmatter.get("updated", "")),
        "path": path.relative_to(root).as_posix(),
    }


def normalize_target(text: str) -> str:
    text = text.split("|", 1)[0]
    text = text.split("#", 1)[0]
    return text.strip().lower()


def strip_code_spans(text):
    """Remove fenced and inline code so [[links]] discussed in code prose
    (not intended as real links) are not parsed as edges."""
    text = FENCED_CODE_RE.sub(" ", text)
    text = INLINE_CODE_RE.sub(" ", text)
    return text


def link_targets(frontmatter, body):
    targets = []
    for item in as_list(frontmatter.get("links")):
        targets.extend(WIKILINK_RE.findall(item))
    targets.extend(WIKILINK_RE.findall(strip_code_spans(body)))
    return targets


def typed_link_targets(frontmatter):
    """(raw_target, edge_type) pairs from depends-on/supersedes frontmatter."""
    typed = []
    for raw in as_list(frontmatter.get("supersedes")):
        typed.extend((t, "supersedes") for t in WIKILINK_RE.findall(raw))
    for raw in as_list(frontmatter.get("depends-on")):
        typed.extend((t, "depends-on") for t in WIKILINK_RE.findall(raw))
    return typed


def tsv_clean(value: str) -> str:
    return value.replace("\t", " ").replace("\r", " ").replace("\n", " ")


def collect_notes(root: Path):
    notes = []
    for sub in NOTE_DIRS:
        directory = root / sub
        if not directory.is_dir():
            continue
        for path in sorted(directory.glob("*.md")):
            try:
                text = path.read_text(encoding="utf-8")
                frontmatter, body = parse_frontmatter(text)
            except (ValueError, OSError) as exc:
                print("journal: skipping malformed note %s: %s" % (path, exc),
                      file=sys.stderr)
                continue
            notes.append((path, frontmatter, body))
    return notes


def build(root: Path):
    notes = collect_notes(root)
    known = {stem_of(path) for (path, _, _) in notes}

    graph = {}
    tags = {}
    unresolved = set()
    rows = []
    nodes = {}
    edges = []
    backlinks = {}

    for (path, frontmatter, body) in notes:
        stem = stem_of(path)
        nodes[stem] = node_record(path, frontmatter, root)
        src_type = nodes[stem]["type"].strip().lower()
        edge_type = "contains" if src_type in CONTAINER_TYPES else "references"

        resolved = []
        edge_best = {}  # target -> best edge type for index.json (precedence)
        for raw in link_targets(frontmatter, body):
            target = normalize_target(raw)
            if not target or target == stem:
                continue
            if target in known:
                if target not in resolved:
                    resolved.append(target)
                edge_best[target] = edge_type
            else:
                unresolved.add((stem, target))
        for raw, etype in typed_link_targets(frontmatter):
            target = normalize_target(raw)
            if not target or target == stem:
                continue
            if target in known:
                if EDGE_PRIORITY[etype] > EDGE_PRIORITY.get(edge_best.get(target), 0):
                    edge_best[target] = etype
            else:
                unresolved.add((stem, target))
        for target in sorted(edge_best):
            edges.append({"from": stem, "to": target, "type": edge_best[target]})
            backlinks.setdefault(target, [])
            if stem not in backlinks[target]:
                backlinks[target].append(stem)
        graph[stem] = sorted(resolved)

        for tag in as_list(frontmatter.get("tags")) + as_list(frontmatter.get("domains")):
            tag = tag.strip().lower()
            if not tag:
                continue
            tags.setdefault(tag, [])
            if stem not in tags[tag]:
                tags[tag].append(stem)

        rows.append((
            stem,
            scalar(frontmatter.get("title", "")),
            scalar(frontmatter.get("summary", "")),
            scalar(frontmatter.get("project", "")),
            scalar(frontmatter.get("helped", "0")),
            scalar(frontmatter.get("updated", "")),
        ))

    if unresolved:
        graph[UNRESOLVED_KEY] = sorted([list(pair) for pair in unresolved])
    tags = {key: sorted(value) for key, value in tags.items()}
    rows.sort()

    index = {
        "nodes": nodes,
        "edges": sorted(edges, key=lambda e: (e["from"], e["to"], e["type"])),
        "backlinks": {key: sorted(value) for key, value in backlinks.items()},
        "unresolved": sorted([list(pair) for pair in unresolved]),
    }
    return graph, tags, rows, index


def write_outputs(root: Path, graph, tags, rows, index):
    index_dir = root / ".index"
    index_dir.mkdir(parents=True, exist_ok=True)
    (index_dir / "graph.json").write_text(
        json.dumps(graph, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (index_dir / "tags.json").write_text(
        json.dumps(tags, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    lines = ["\t".join(tsv_clean(col) for col in row) for row in rows]
    text = "\n".join(lines) + ("\n" if lines else "")
    (index_dir / "summaries.tsv").write_text(text, encoding="utf-8")
    (index_dir / "index.json").write_text(
        json.dumps(index, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main(argv) -> int:
    root = vault_root()
    graph, tags, rows, index = build(root)
    write_outputs(root, graph, tags, rows, index)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
