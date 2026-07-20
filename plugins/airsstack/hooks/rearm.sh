#!/bin/sh
# airsstack enforcement re-arm launcher.
#
# Same D10 rule as enforce.sh: never propagate a child's status, always exit 0.
# Deliberately no `exec`, which would replace the shell and hand python's
# status straight back to Claude Code.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0

if command -v python3 >/dev/null 2>&1 && [ -r "$DIR/rearm.py" ]; then
  python3 "$DIR/rearm.py" || exit 0
fi

exit 0
