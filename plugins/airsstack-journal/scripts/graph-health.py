#!/usr/bin/env python3
"""Deterministic graph-health report over the airsstack-journal index.

Reads .index/index.json and reports three signals with NO model and NO writes:
  orphans — nodes with zero in+out edges (excluding type: daily containers)
  hubs    — nodes whose total degree (in+out) >= AIRSSTACK_JOURNAL_HUB_DEGREE
            (default 12)
  broken  — unresolved [stem, missing-target] pairs

Emits a human Markdown section plus a fenced ```health JSON block the curator
parses. Absent/empty/malformed index -> empty report, exit 0 (fail-open).
"""
import json
import os
import sys
from pathlib import Path

DEFAULT_HUB_DEGREE = 12


def vault_root() -> Path:
    home = os.environ.get("AIRSSTACK_HOME") or os.path.expanduser("~/.airsstack")
    return Path(home) / "journal"


def hub_degree() -> int:
    try:
        val = int(os.environ.get("AIRSSTACK_JOURNAL_HUB_DEGREE", ""))
        return val if val > 0 else DEFAULT_HUB_DEGREE
    except ValueError:
        return DEFAULT_HUB_DEGREE


def analyze(index):
    nodes = index.get("nodes", {})
    edges = index.get("edges", [])
    unresolved = index.get("unresolved", [])

    degree = {stem: 0 for stem in nodes}
    for e in edges:
        if e.get("from") in degree:
            degree[e["from"]] += 1
        if e.get("to") in degree:
            degree[e["to"]] += 1

    threshold = hub_degree()
    orphans = sorted(
        stem for stem, d in degree.items()
        if d == 0 and nodes[stem].get("type", "") != "daily"
    )
    hubs = sorted(
        (stem for stem, d in degree.items() if d >= threshold),
        key=lambda s: (-degree[s], s),
    )
    return {
        "orphans": orphans,
        "hubs": [{"stem": s, "degree": degree[s]} for s in hubs],
        "broken": sorted([list(p) for p in unresolved]),
    }


def render(report) -> str:
    lines = ["# Journal graph-health report", "", "## Orphans (no links in or out)"]
    lines += (["- [[%s]]" % s for s in report["orphans"]]
              if report["orphans"] else ["_none_"])
    lines += ["", "## Hubs (over-connected)"]
    lines += (["- [[%s]] — degree %d" % (h["stem"], h["degree"]) for h in report["hubs"]]
              if report["hubs"] else ["_none_"])
    lines += ["", "## Broken links"]
    lines += (["- [[%s]] → %s (missing)" % (a, b) for a, b in report["broken"]]
              if report["broken"] else ["_none_"])
    lines += ["", "```health", json.dumps(report, indent=2, sort_keys=True), "```"]
    return "\n".join(lines) + "\n"


def main(argv) -> int:
    index_path = vault_root() / ".index" / "index.json"
    try:
        index = json.loads(index_path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        print(render({"orphans": [], "hubs": [], "broken": []}), end="")
        return 0
    if not isinstance(index, dict):
        index = {}
    print(render(analyze(index)), end="")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
