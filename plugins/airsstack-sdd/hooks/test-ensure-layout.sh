#!/usr/bin/env bash
# Test harness for ensure-layout.sh — verifies the two-root SDD layout:
#   rfcs/ stays worktree-local; specs/plans/_archive go HOME-global under a
#   stable per-repo key. Run from anywhere: sh test-ensure-layout.sh
set -u

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
ENSURE="$SCRIPT_DIR/ensure-layout.sh"

fail=0
check() { # $1 = exit status of the assertion, $2 = description
  if [ "$1" = "0" ]; then
    printf 'ok   - %s\n' "$2"
  else
    printf 'FAIL - %s\n' "$2"
    fail=1
  fi
}

# Isolated sandbox: temp HOME-global root + temp git repo as cwd.
work=$(mktemp -d)
home=$(mktemp -d)
export AIRSSTACK_HOME="$home"
trap 'rm -rf "$work" "$home"' EXIT

repo="$work/myrepo"
mkdir -p "$repo"
( cd "$repo" && git init -q && \
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m init )

# Expected key, computed the same way the script does (from main worktree: .git).
# pwd -P is required to canonicalize symlinked paths (e.g. macOS /var -> /private/var).
abs=$(cd "$repo" && pwd -P)/.git
base=$(basename "$(dirname "$abs")")
base=$(printf '%s' "$base" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')
hash8=$(printf '%s' "$abs" | shasum | cut -c1-8)
key="${base}-${hash8}"
home_root="$home/cc/plugins/sdd/$key"

# --- Case 1: first run creates both roots correctly ---
( cd "$repo" && sh "$ENSURE" >/dev/null )
[ -d "$repo/.airsstack/cc/plugins/sdd/rfcs" ];        check $? "rfcs/ created worktree-local"
[ -d "$home_root/specs" ];                            check $? "specs/ created HOME-global under key"
[ -d "$home_root/plans" ];                            check $? "plans/ created HOME-global under key"
[ -d "$home_root/plans/_archive" ];                   check $? "plans/_archive/ created HOME-global"
[ ! -d "$repo/.airsstack/cc/plugins/sdd/specs" ];     check $? "specs/ NOT created worktree-local"

# --- Case 2: gitignore scoped to the worktree-local tree only ---
grep -qxF '.airsstack/' "$repo/.gitignore";           check $? ".gitignore has .airsstack/"

# --- Case 3: idempotency ---
out=$( cd "$repo" && sh "$ENSURE" )
printf '%s' "$out" | grep -q 'already present';       check $? "second run is a no-op"

# --- Case 4: key stable across a linked worktree (one store dir, no fragmentation) ---
wt="$work/wt-feature"
( cd "$repo" && git worktree add -q "$wt" -b feature )
( cd "$wt" && sh "$ENSURE" >/dev/null )
n=$(find "$home/cc/plugins/sdd" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')
[ "$n" = "1" ];                                       check $? "linked worktree resolves to same key"

# --- Case 5: no-git fallback hashes the cwd ---
nogit="$work/plain"
mkdir -p "$nogit"
( cd "$nogit" && sh "$ENSURE" >/dev/null )
fb_abs=$(cd "$nogit" && pwd -P)
fb_base=$(basename "$fb_abs")
fb_base=$(printf '%s' "$fb_base" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')
fb_hash=$(printf '%s' "$fb_abs" | shasum | cut -c1-8)
[ -d "$home/cc/plugins/sdd/${fb_base}-${fb_hash}/specs" ]; check $? "no-git falls back to cwd-hash key"

# --- Case 6: basename sanitizer replaces disallowed chars with '-' ---
rawbase="plain@test"
sandir="$work/$rawbase"
mkdir -p "$sandir"
( cd "$sandir" && sh "$ENSURE" >/dev/null )
san_abs=$(cd "$sandir" && pwd -P)
san_base=$(basename "$san_abs")
san_base_sanitized=$(printf '%s' "$san_base" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')
san_hash=$(printf '%s' "$san_abs" | shasum | cut -c1-8)
san_key="${san_base_sanitized}-${san_hash}"
[ -d "$home/cc/plugins/sdd/${san_key}/specs" ];           check $? "sanitizer: disallowed char '@' replaced by '-' in key base"

if [ "$fail" = "0" ]; then printf '\nALL PASS\n'; else printf '\nFAILURES\n'; exit 1; fi
