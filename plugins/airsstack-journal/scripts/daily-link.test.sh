#!/bin/sh
# Tests for daily-link.sh — creates the daily note, links idempotently.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export AIRSSTACK_HOME="$TMP"
FILE="$TMP/journal/daily/2026-06-23.md"

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

# 1. First link creates the daily note with frontmatter + the link.
sh "$SCRIPT_DIR/daily-link.sh" 2026-06-23 tokio-cancellation-safety
check $? "first run exits 0"
[ -f "$FILE" ]; check $? "creates daily note"
grep -q '^type: daily$' "$FILE"; check $? "daily note has type: daily"
grep -qF '[[tokio-cancellation-safety]]' "$FILE"; check $? "links the stem"

# 2. Re-linking the same stem is idempotent (exactly one occurrence).
sh "$SCRIPT_DIR/daily-link.sh" 2026-06-23 tokio-cancellation-safety
n=$(grep -cF '[[tokio-cancellation-safety]]' "$FILE")
[ "$n" -eq 1 ]; check $? "idempotent: one link after double-add (got $n)"

# 3. A second distinct stem also appears.
sh "$SCRIPT_DIR/daily-link.sh" 2026-06-23 session-abc12345
grep -qF '[[session-abc12345]]' "$FILE"; check $? "second distinct stem appears"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
