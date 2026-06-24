#!/bin/sh
# Tests for session-start.sh — provision + staleness-gated rebuild, fail-open.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export AIRSSTACK_HOME="$TMP"
MARKER="$TMP/journal/.index/summaries.tsv"

check() {
  if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"
  else printf 'FAIL: %s\n' "$2"; fail=1; fi
}

# 1. First run: no marker → provisions and builds the index.
sh "$SCRIPT_DIR/session-start.sh" >/dev/null 2>&1
check $? "first run exits 0"
[ -d "$TMP/journal/notes" ]; check $? "provisions vault"
[ -f "$MARKER" ]; check $? "builds index marker on first run"

# 2. Add a note newer than the marker → rebuild picks it up.
sleep 1
printf -- '---\ntitle: Fresh\nsummary: s\n---\n' > "$TMP/journal/notes/fresh.md"
sh "$SCRIPT_DIR/session-start.sh" >/dev/null 2>&1
check $? "rebuild run exits 0"
grep -q '^fresh	' "$MARKER"; check $? "stale rebuild indexes the new note"

# 3. Fail-open when the index build fails: a python3 that errors must not block.
#    (Emptying PATH would also break mkdir/find, so shadow a failing python3
#     stub onto PATH while real coreutils stay resolvable.)
TMP2=$(mktemp -d); export AIRSSTACK_HOME="$TMP2"
STUBBIN=$(mktemp -d)
printf '#!/bin/sh\nexit 1\n' > "$STUBBIN/python3"
chmod +x "$STUBBIN/python3"
PATH="$STUBBIN:$PATH" sh "$SCRIPT_DIR/session-start.sh" >/dev/null 2>&1
check $? "fails open when build errors (exit 0)"
[ -d "$TMP2/journal/notes" ]; check $? "still provisions when build errors"
rm -rf "$TMP2" "$STUBBIN"

# 4. Orientation card: a project note surfaces in the SessionStart card as
#    additionalContext. The note's project must match project-key.sh output.
TMP3=$(mktemp -d); export AIRSSTACK_HOME="$TMP3"
PROJ=$(sh "$SCRIPT_DIR/project-key.sh")
mkdir -p "$TMP3/journal/notes"
cat > "$TMP3/journal/notes/recall-card-note.md" <<EOF
---
title: Card Note
type: insight
project: $PROJ
summary: shows up in the card
updated: 2026-06-24 12:00
---
body
EOF
out=$(sh "$SCRIPT_DIR/session-start.sh" 2>/dev/null)
printf '%s' "$out" | grep -q 'additionalContext'; check $? "emits additionalContext when card present"
printf '%s' "$out" | grep -q 'recall-card-note'; check $? "card lists the project note"
rm -rf "$TMP3"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0
else printf 'FAILURES\n'; exit 1; fi
