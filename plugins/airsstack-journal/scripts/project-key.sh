#!/bin/sh
# Resolve the human-readable `project` floor for a journal note: the
# repository basename. Linked worktrees collapse to the main repo (via
# git-common-dir); no git falls back to the cwd basename. The token is
# sanitised so it is safe as a frontmatter scalar. Deterministic, no side
# effects, always prints one line, always exits 0.
set -u

if common_dir=$(git rev-parse --git-common-dir 2>/dev/null); then
  abs=$(cd "$(dirname "$common_dir")" 2>/dev/null && pwd -P)/$(basename "$common_dir")
  base=$(basename "$(dirname "$abs")")
else
  base=$(basename "$(pwd -P)")
fi

base=$(printf '%s' "$base" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')
printf '%s\n' "$base"
