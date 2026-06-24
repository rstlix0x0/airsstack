#!/bin/sh
# Tests for project-key.sh — repo basename floor; worktrees collapse; no-git fallback.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

# 1. Inside a git repo → repo basename.
mkdir -p "$TMP/myrepo"
( cd "$TMP/myrepo" && git init -q )
out=$( cd "$TMP/myrepo" && sh "$SCRIPT_DIR/project-key.sh" )
[ "$out" = "myrepo" ]; check $? "git repo prints repo basename (got '$out')"

# 2. From a subdirectory → still the repo basename.
mkdir -p "$TMP/myrepo/crates/sub"
out=$( cd "$TMP/myrepo/crates/sub" && sh "$SCRIPT_DIR/project-key.sh" )
[ "$out" = "myrepo" ]; check $? "subdir prints repo basename (got '$out')"

# 3. Linked worktree → same main-repo basename.
( cd "$TMP/myrepo" \
    && git -c user.email=t@t -c user.name=t commit -q --allow-empty -m init \
    && git worktree add -q "$TMP/wt-feature" 2>/dev/null )
out=$( cd "$TMP/wt-feature" && sh "$SCRIPT_DIR/project-key.sh" )
[ "$out" = "myrepo" ]; check $? "worktree collapses to main repo basename (got '$out')"

# 4. No git → cwd basename.
mkdir -p "$TMP/plain"
out=$( cd "$TMP/plain" && sh "$SCRIPT_DIR/project-key.sh" )
[ "$out" = "plain" ]; check $? "no-git prints cwd basename (got '$out')"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
