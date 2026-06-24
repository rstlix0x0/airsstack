#!/bin/sh
# Tests for provision.sh — runs against a temp AIRSSTACK_HOME.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export AIRSSTACK_HOME="$TMP"

check() { # $1 = exit status of the assertion, $2 = description
  if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"
  else printf 'FAIL: %s\n' "$2"; fail=1; fi
}

sh "$SCRIPT_DIR/provision.sh" >/dev/null 2>&1
check $? "first run exits 0"

for d in daily sessions notes mocs .index; do
  [ -d "$TMP/journal/$d" ]; check $? "creates $d"
done

[ ! -e "$TMP/journal/.gitignore" ]; check $? "no .gitignore created"
[ ! -e "$TMP/journal/.obsidian" ]; check $? "no .obsidian created"

sh "$SCRIPT_DIR/provision.sh" >/dev/null 2>&1
check $? "idempotent re-run exits 0"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0
else printf 'FAILURES\n'; exit 1; fi
