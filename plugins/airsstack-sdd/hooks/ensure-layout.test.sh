#!/usr/bin/env sh
# Idempotency contract test for ensure-layout.sh.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="$SCRIPT_DIR/ensure-layout.sh"

fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"

# First run provisions everything.
sh "$SCRIPT" >/dev/null

for d in rfcs specs plans plans/_archive; do
  [ -d ".airsstack/cc/plugins/sdd/$d" ] || fail "missing dir $d after first run"
done
[ -f .gitignore ] || fail ".gitignore not created"
count=$(grep -cxF '.airsstack/' .gitignore)
[ "$count" -eq 1 ] || fail ".gitignore should have exactly one .airsstack/ line, got $count"

# Second run is a no-op: gitignore line count stays 1.
sh "$SCRIPT" >/dev/null
count=$(grep -cxF '.airsstack/' .gitignore)
[ "$count" -eq 1 ] || fail "second run duplicated .gitignore line, got $count"

# Pre-existing .gitignore without the line gets the line appended, content preserved.
work2=$(mktemp -d)
cd "$work2"
printf 'target/\n' > .gitignore
sh "$SCRIPT" >/dev/null
grep -qxF 'target/' .gitignore || fail "pre-existing .gitignore content lost"
grep -qxF '.airsstack/' .gitignore || fail ".airsstack/ not appended to existing .gitignore"
rm -rf "$work2"

printf 'PASS\n'
