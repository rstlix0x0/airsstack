#!/bin/sh
# airsstack enforcement re-arm launcher.
#
# Same rule as enforce.sh: never propagate a child's status, always exit 0. Deliberately no `exec`,
# which would replace the shell and hand airsl's status straight back to Claude Code.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0
command -v airsl >/dev/null 2>&1 || exit 0

SENTINELS="${TMPDIR:-/tmp}"

airsl run --fail-open --policy confined \
  --allow-env TMPDIR \
  --allow-read "$SENTINELS" --allow-write "$SENTINELS" \
  "$DIR/rearm.lua" || exit 0

exit 0
