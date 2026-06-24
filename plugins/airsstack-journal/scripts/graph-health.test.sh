#!/bin/sh
# Tests for graph-health.py — orphan/hub/broken detection over a fixture index.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export AIRSSTACK_HOME="$TMP"
export AIRSSTACK_JOURNAL_HUB_DEGREE=3
mkdir -p "$TMP/journal/.index"
IDX="$TMP/journal/.index/index.json"

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

cat > "$IDX" <<'EOF'
{
  "nodes": {
    "hub": {"type": "insight"},
    "a": {"type": "insight"},
    "b": {"type": "insight"},
    "c": {"type": "insight"},
    "orphan": {"type": "insight"},
    "today": {"type": "daily"}
  },
  "edges": [
    {"from": "hub", "to": "a", "type": "references"},
    {"from": "hub", "to": "b", "type": "references"},
    {"from": "hub", "to": "c", "type": "references"}
  ],
  "backlinks": {},
  "unresolved": [["a", "ghost"]]
}
EOF

OUT=$(python3 "$SCRIPT_DIR/graph-health.py")
printf '%s' "$OUT" | grep -q '\[\[orphan\]\]'; check $? "orphan flagged"
printf '%s' "$OUT" | grep -q '\[\[hub\]\] — degree 3'; check $? "hub flagged at threshold"
printf '%s' "$OUT" | grep -q 'ghost (missing)'; check $? "broken link flagged"
if printf '%s' "$OUT" | grep -q '\[\[today\]\]'; then daily=1; else daily=0; fi
[ "$daily" -eq 0 ]; check $? "daily-typed node not flagged orphan"

# Absent index -> empty report, exit 0 (fail-open).
rm -f "$IDX"
python3 "$SCRIPT_DIR/graph-health.py" >/dev/null 2>&1; check $? "absent index exits 0"

# Non-dict valid JSON (e.g. array) -> empty report, exit 0 (fail-open).
printf '[]\n' > "$IDX"
python3 "$SCRIPT_DIR/graph-health.py" >/dev/null 2>&1; check $? "non-dict valid JSON exits 0"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
