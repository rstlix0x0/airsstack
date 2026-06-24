#!/bin/sh
# Tests for orientation.sh — project-scoped recent-activity card from summaries.tsv.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export AIRSSTACK_HOME="$TMP"
mkdir -p "$TMP/journal/.index"
TSV="$TMP/journal/.index/summaries.tsv"
TAB=$(printf '\t')

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

# Seed summaries.tsv: stem \t title \t summary \t project \t helped \t updated
{
  printf 'session-aaaa1111%sS1%sfirst session%sclauders%s0%s2026-06-20 09:00\n' "$TAB" "$TAB" "$TAB" "$TAB" "$TAB"
  printf 'session-bbbb2222%sS2%ssecond session%sclauders%s0%s2026-06-23 09:00\n' "$TAB" "$TAB" "$TAB" "$TAB" "$TAB"
  printf 'tokio-cancel%sTokio%scancel safety%sclauders%s2%s2026-06-22 10:00\n' "$TAB" "$TAB" "$TAB" "$TAB" "$TAB"
  printf 'other-note%sOther%snot mine%sopenrouter-rs%s0%s2026-06-24 10:00\n' "$TAB" "$TAB" "$TAB" "$TAB" "$TAB"
} > "$TSV"

card=$(sh "$SCRIPT_DIR/orientation.sh" clauders)

printf '%s\n' "$card" | grep -q '\[\[session-bbbb2222\]\]'; check $? "lists a project session"
printf '%s\n' "$card" | grep -q '\[\[tokio-cancel\]\]'; check $? "lists a project note"
if printf '%s\n' "$card" | grep -q 'other-note'; then r=1; else r=0; fi
check $r "excludes a different project's note"

# Empty/absent tsv → empty output, exit 0.
rm -f "$TSV"
out=$(sh "$SCRIPT_DIR/orientation.sh" clauders)
[ -z "$out" ]; check $? "absent tsv yields empty card"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
