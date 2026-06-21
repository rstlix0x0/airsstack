#!/usr/bin/env sh
# Single source of truth (shell side) for the SDD artifact tree location.
# The prose mirror is plugins/airsstack-sdd/references/artifact-paths.md; the two
# MUST agree. Change one, change the other.
#
# Two roots, split by artifact type:
#   - rfcs/ : worktree-local, transient input, under the git-ignored .airsstack/ tree.
#   - specs/, plans/, plans/_archive/ : HOME-global, durable, shared across every
#     worktree of one repo, keyed by a stable per-repo project key.
#
# Idempotent: creates only what is missing; never duplicates the .gitignore line.
set -eu

# --- Worktree-local root (rfcs only), relative to the consuming project root. ---
RFC_LOCAL_ROOT=".airsstack/cc/plugins/sdd"

# --- HOME-global base. Honors AIRSSTACK_HOME, defaulting to ~/.airsstack — the same
#     contract the snapshot and memory stores use. ---
AIRSSTACK_HOME="${AIRSSTACK_HOME:-$HOME/.airsstack}"

# Stable per-repo project key: every linked worktree resolves to the main repo's
# git-common-dir, so all worktrees collapse to one store. No git -> hash the cwd.
# pwd -P canonicalizes symlinked paths (e.g. macOS /var -> /private/var) so the
# main and linked worktrees hash to one key. Portable; no OS-specific literal.
if common_dir=$(git rev-parse --git-common-dir 2>/dev/null); then
  abs=$(cd "$(dirname "$common_dir")" 2>/dev/null && pwd -P)/$(basename "$common_dir")
  base=$(basename "$(dirname "$abs")")
else
  abs=$(pwd -P)
  base=$(basename "$abs")
fi
# Sanitize the human-readable component only; hash8 (from the full path) keeps
# keys unique even if sanitization collapses distinct names.
base=$(printf '%s' "$base" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')
hash8=$(printf '%s' "$abs" | shasum | cut -c1-8)
key="${base}-${hash8}"

HOME_ROOT="${AIRSSTACK_HOME}/cc/plugins/sdd/${key}"

created=""

# Worktree-local: rfcs/ only.
for d in "$RFC_LOCAL_ROOT/rfcs"; do
  if [ ! -d "$d" ]; then
    mkdir -p "$d"
    created="${created}  created ${d}
"
  fi
done

# HOME-global: specs/, plans/, plans/_archive/.
for d in "$HOME_ROOT/specs" "$HOME_ROOT/plans" "$HOME_ROOT/plans/_archive"; do
  if [ ! -d "$d" ]; then
    mkdir -p "$d"
    created="${created}  created ${d}
"
  fi
done

# Ensure .gitignore ignores the worktree-local .airsstack/ tree, exactly once.
# Only this root needs it; the HOME-global root is outside any repo and cannot leak.
if [ ! -f .gitignore ]; then
  printf '.airsstack/\n' > .gitignore
  created="${created}  created .gitignore with .airsstack/
"
elif ! grep -qxF '.airsstack/' .gitignore; then
  printf '.airsstack/\n' >> .gitignore
  created="${created}  appended .airsstack/ to .gitignore
"
fi

if [ -n "$created" ]; then
  printf 'airsstack-sdd layout provisioned:\n'
  printf '%s' "$created"
else
  printf 'airsstack-sdd layout already present; nothing to do.\n'
fi
