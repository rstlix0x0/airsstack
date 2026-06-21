#!/usr/bin/env sh
# Context Handoff session manager for the airsstack orchestration.
# Single source of truth (shell side) for the handoff tree path, the session
# liveness lease, and pruning. Prose mirror:
#   plugins/airsstack/skills/process-guidelines/references/context-handoff.md
# The two MUST agree. Change one, change the other.
#
# Subcommands:
#   init                 resolve base, mint a session, write .active, prune, print dir+id
#   beat <session-dir>   refresh the session's .active lease (heartbeat)
#   end  <session-dir>   remove the session's .active lease (clean close)
set -eu

HANDOFF_REL=".airsstack/cc/plugins/airsstack/handoff"
KEEP="${AIRSSTACK_HANDOFF_KEEP:-10}"
GRACE_MIN="${AIRSSTACK_HANDOFF_GRACE:-120}"   # minutes; lease staleness threshold

worktree_root() {
  git rev-parse --show-toplevel 2>/dev/null || pwd
}

resolve_base() {
  printf '%s/%s' "$(worktree_root)" "$HANDOFF_REL"
}

prune() {
  base="$1"
  [ -d "$base" ] || return 0
  names=$(ls -1 "$base" 2>/dev/null | sort)   # ascending; timestamp prefix => lexical = chronological
  total=0
  for name in $names; do
    [ -d "$base/$name" ] || continue
    total=$((total + 1))
  done
  n_candidates=$((total - KEEP))
  [ "$n_candidates" -lt 0 ] && n_candidates=0
  [ "$n_candidates" -eq 0 ] && return 0
  i=0
  for name in $names; do
    [ -d "$base/$name" ] || continue
    i=$((i + 1))
    [ "$i" -le "$n_candidates" ] || break   # only the oldest n_candidates are prune candidates
    dir="$base/$name"
    if [ ! -e "$dir/.active" ]; then
      rm -rf "$dir"
    elif [ -n "$(find "$dir/.active" -mmin +"$GRACE_MIN" 2>/dev/null)" ]; then
      rm -rf "$dir"   # lease older than the grace window
    fi
  done
}

ensure_gitignore() {
  gi="$(worktree_root)/.gitignore"
  if [ ! -f "$gi" ]; then
    printf '.airsstack/\n' > "$gi"
  elif ! grep -qxF '.airsstack/' "$gi"; then
    printf '.airsstack/\n' >> "$gi"
  fi
}

cmd_init() {
  base=$(resolve_base)
  ensure_gitignore
  ts=$(date +%Y%m%d-%H%M%S)
  rand=$(head -c2 /dev/urandom | od -An -tx1 | tr -d ' \n')
  session_id="${ts}-${rand}"
  session_dir="$base/$session_id"
  mkdir -p "$session_dir"
  touch "$session_dir/.active"
  prune "$base"
  printf '%s\n' "$session_dir"
  printf '%s\n' "$session_id"
}

cmd_beat() {
  [ "$#" -ge 1 ] || { printf 'beat: missing <session-dir>\n' >&2; exit 2; }
  dir="$1"
  [ -d "$dir" ] || { printf 'beat: no such session dir: %s\n' "$dir" >&2; exit 1; }
  touch "$dir/.active"
}

cmd_end() {
  [ "$#" -ge 1 ] || { printf 'end: missing <session-dir>\n' >&2; exit 2; }
  rm -f "$1/.active"
}

case "${1:-}" in
  init) shift; cmd_init "$@" ;;
  beat) shift; cmd_beat "$@" ;;
  end)  shift; cmd_end "$@" ;;
  *) printf 'usage: handoff.sh {init|beat <dir>|end <dir>}\n' >&2; exit 2 ;;
esac
