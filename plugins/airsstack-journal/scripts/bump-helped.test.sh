#!/bin/sh
# Tests for bump-helped.sh — increment a note's helped: counter, rebuild index.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export AIRSSTACK_HOME="$TMP"
mkdir -p "$TMP/journal/notes" "$TMP/journal/.index"
NOTE="$TMP/journal/notes/alpha.md"

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

cat > "$NOTE" <<'EOF'
---
title: Alpha
type: insight
helped: 0
updated: 2026-06-23 10:00
---
body
EOF

# 1. First bump 0 -> 1, updated untouched.
sh "$SCRIPT_DIR/bump-helped.sh" alpha >/dev/null 2>&1
check $? "bump exits 0"
grep -q '^helped: 1$' "$NOTE"; check $? "helped incremented to 1"
grep -q '^updated: 2026-06-23 10:00$' "$NOTE"; check $? "updated unchanged"

# 2. Second bump 1 -> 2.
sh "$SCRIPT_DIR/bump-helped.sh" alpha >/dev/null 2>&1
grep -q '^helped: 2$' "$NOTE"; check $? "helped incremented to 2"

# 3. Case-insensitive stem resolution.
sh "$SCRIPT_DIR/bump-helped.sh" ALPHA >/dev/null 2>&1
grep -q '^helped: 3$' "$NOTE"; check $? "case-insensitive stem resolves"

# 4. Index refreshed: index.json reflects helped: 3 (python3 present).
if command -v python3 >/dev/null 2>&1; then
  grep -q '"helped": 3' "$TMP/journal/.index/index.json"; check $? "index.json reflects bumped helped"
fi

# 5. Missing stem exits nonzero.
sh "$SCRIPT_DIR/bump-helped.sh" ghost >/dev/null 2>&1
[ $? -ne 0 ]; check $? "missing stem exits nonzero"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
