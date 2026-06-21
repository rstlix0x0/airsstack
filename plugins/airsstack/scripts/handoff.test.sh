#!/usr/bin/env sh
# Contract tests for handoff.sh.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="$SCRIPT_DIR/handoff.sh"
BASE_REL=".airsstack/cc/plugins/airsstack/handoff"

fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
newwork() { d=$(mktemp -d); cd "$d" && pwd -P; }

# --- init: creates a leased session dir under the worktree-root handoff tree ---
work=$(newwork); cd "$work"; git init -q
out=$(sh "$SCRIPT" init)
sdir=$(printf '%s\n' "$out" | sed -n 1p)
sid=$(printf '%s\n' "$out" | sed -n 2p)
[ -d "$sdir" ] || fail "init did not create the session dir"
[ -e "$sdir/.active" ] || fail "init did not write the .active lease"
case "$sdir" in
  "$work/$BASE_REL"/*) ;;
  *) fail "session dir not under worktree handoff root: $sdir" ;;
esac
printf '%s' "$sid" | grep -Eq '^[0-9]{8}-[0-9]{6}-[0-9a-f]{4}$' \
  || fail "session id not <YYYYMMDD-HHMMSS-rand4>: $sid"
grep -qxF '.airsstack/' .gitignore || fail "init did not ensure the .airsstack/ ignore line"
rm -rf "$work"

# --- init: no git repo -> base falls back to cwd ---
work=$(newwork); cd "$work"
out=$(sh "$SCRIPT" init)
sdir=$(printf '%s\n' "$out" | sed -n 1p)
case "$sdir" in
  "$work/$BASE_REL"/*) ;;
  *) fail "no-git fallback base wrong: $sdir" ;;
esac
rm -rf "$work"

# --- beat: creates/refreshes the lease; fails on a missing dir ---
work=$(newwork); cd "$work"; git init -q
out=$(sh "$SCRIPT" init); sdir=$(printf '%s\n' "$out" | sed -n 1p)
rm -f "$sdir/.active"
sh "$SCRIPT" beat "$sdir"
[ -e "$sdir/.active" ] || fail "beat did not (re)create the .active lease"
if sh "$SCRIPT" beat "$work/does-not-exist" 2>/dev/null; then
  fail "beat should fail on a missing session dir"
fi

# --- end: removes the lease ---
sh "$SCRIPT" end "$sdir"
[ -e "$sdir/.active" ] && fail "end did not remove the .active lease"
rm -rf "$work"

# --- prune: beyond keep-N AND lease stale/absent -> dropped; fresh lease -> kept ---
work=$(newwork); cd "$work"; git init -q
base="$BASE_REL"
mkdir -p "$base/20200101-000000-aaaa"; touch -t 202001010000 "$base/20200101-000000-aaaa/.active"  # old, stale lease
mkdir -p "$base/20200101-000001-bbbb"; touch    "$base/20200101-000001-bbbb/.active"               # old, fresh lease
mkdir -p "$base/20200101-000002-cccc"                                                               # old, NO lease
AIRSSTACK_HANDOFF_KEEP=1 AIRSSTACK_HANDOFF_GRACE=120 sh "$SCRIPT" init >/dev/null
[ -d "$base/20200101-000000-aaaa" ] && fail "stale-lease old session was not pruned"
[ -d "$base/20200101-000002-cccc" ] && fail "lease-less old session was not pruned"
[ -d "$base/20200101-000001-bbbb" ] || fail "fresh-lease old session was wrongly pruned"
rm -rf "$work"

# --- prune: KEEP override protects everything ---
work=$(newwork); cd "$work"; git init -q
base="$BASE_REL"
mkdir -p "$base/20200101-000000-aaaa"; touch -t 202001010000 "$base/20200101-000000-aaaa/.active"
AIRSSTACK_HANDOFF_KEEP=5 sh "$SCRIPT" init >/dev/null
[ -d "$base/20200101-000000-aaaa" ] || fail "KEEP=5 should protect the old dir (not beyond keep-N)"
rm -rf "$work"

# --- prune: GRACE override keeps a stale-by-time lease ---
work=$(newwork); cd "$work"; git init -q
base="$BASE_REL"
mkdir -p "$base/20200101-000000-aaaa"; touch -t 202001010000 "$base/20200101-000000-aaaa/.active"
AIRSSTACK_HANDOFF_KEEP=1 AIRSSTACK_HANDOFF_GRACE=999999999 sh "$SCRIPT" init >/dev/null
[ -d "$base/20200101-000000-aaaa" ] || fail "huge GRACE should keep the dir (lease counts fresh)"
rm -rf "$work"

# --- prune: within keep-N is kept regardless of a stale lease (spec §12) ---
work=$(newwork); cd "$work"; git init -q
base="$BASE_REL"
mkdir -p "$base/20200101-000000-aaaa"; touch -t 202001010000 "$base/20200101-000000-aaaa/.active"  # stale lease, but within keep-N
AIRSSTACK_HANDOFF_GRACE=120 sh "$SCRIPT" init >/dev/null   # default KEEP=10: aaaa + new session = 2 dirs, within N
[ -d "$base/20200101-000000-aaaa" ] || fail "within keep-N must be kept regardless of a stale lease (spec §12)"
rm -rf "$work"

printf 'PASS\n'
