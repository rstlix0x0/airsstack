#!/bin/sh
# Tests for transcript-path.sh — slug munge, CLAUDE_CONFIG_DIR, pwd -P fallback, absent.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export CLAUDE_CONFIG_DIR="$TMP/cfg"
SID="bfe14b79-c132-40c2-8dc5-da62f3658227"

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

# Seed a fake transcript under the slug of the current working dir.
slug=$(printf '%s' "$(pwd)" | LC_ALL=C tr -c 'A-Za-z0-9' '-')
mkdir -p "$CLAUDE_CONFIG_DIR/projects/$slug"
: > "$CLAUDE_CONFIG_DIR/projects/$slug/$SID.jsonl"

# 1. Resolves the path honouring CLAUDE_CONFIG_DIR.
out=$(sh "$SCRIPT_DIR/transcript-path.sh" "$SID"); rc=$?
[ "$rc" -eq 0 ]; check $? "exit 0 when transcript exists"
[ "$out" = "$CLAUDE_CONFIG_DIR/projects/$slug/$SID.jsonl" ]; check $? "prints the resolved path"
case "$out" in */"$SID".jsonl) ok=0;; *) ok=1;; esac; check "$ok" "path ends in <session_id>.jsonl"

# 2. Slug munge: no '/' or '.' survive.
case "$slug" in *[/.]*) clean=1;; *) clean=0;; esac; check "$clean" "slug has no '/' or '.'"

# 3. Absent transcript -> nothing, non-zero.
out2=$(sh "$SCRIPT_DIR/transcript-path.sh" "no-such-session"); rc2=$?
[ "$rc2" -ne 0 ]; check $? "non-zero when absent"
[ -z "$out2" ]; check $? "prints nothing when absent"

# 4. Missing arg -> non-zero.
sh "$SCRIPT_DIR/transcript-path.sh" >/dev/null 2>&1; [ $? -ne 0 ]; check $? "non-zero with no session id"

# 5. Falls back to the pwd -P slug when the logical-cwd slug dir is absent.
mkdir -p "$TMP/real"; ln -s "$TMP/real" "$TMP/link"
realslug=$(cd "$TMP/link" && printf '%s' "$(pwd -P)" | LC_ALL=C tr -c 'A-Za-z0-9' '-')
mkdir -p "$CLAUDE_CONFIG_DIR/projects/$realslug"
: > "$CLAUDE_CONFIG_DIR/projects/$realslug/$SID.jsonl"
out5=$(cd "$TMP/link" && sh "$SCRIPT_DIR/transcript-path.sh" "$SID"); rc5=$?
{ [ "$rc5" -eq 0 ] && [ "$out5" = "$CLAUDE_CONFIG_DIR/projects/$realslug/$SID.jsonl" ]; }
check $? "falls back to pwd -P slug"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
