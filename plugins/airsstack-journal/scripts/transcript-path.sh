#!/bin/sh
# Resolve the Claude Code transcript JSONL path for a session id.
# Usage: transcript-path.sh <session_id>
# Prints the transcript path and exits 0 when it exists; prints nothing and
# exits 1 when no transcript can be located. The store slug is the working
# directory with every non-alphanumeric character replaced by '-' (the same
# munge Claude Code uses for ~/.claude/projects/<slug>/). Tries the logical
# cwd first, then the symlink-resolved pwd -P. Honours CLAUDE_CONFIG_DIR.
set -u

[ "$#" -ge 1 ] || exit 1
session_id="$1"
[ -n "$session_id" ] || exit 1

config_dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"

for dir in "$(pwd)" "$(pwd -P)"; do
  slug=$(printf '%s' "$dir" | LC_ALL=C tr -c 'A-Za-z0-9' '-')
  path="$config_dir/projects/$slug/$session_id.jsonl"
  if [ -f "$path" ]; then
    printf '%s\n' "$path"
    exit 0
  fi
done

exit 1
