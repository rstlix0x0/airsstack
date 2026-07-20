#!/bin/sh
# airsstack-plugin-dev cache delivery guard launcher.
#
# Same rule as the dispatcher's launcher: never propagate a child's status.
# Deliberately no `exec`, which would hand python's status back to Claude Code.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0

if command -v python3 >/dev/null 2>&1 && [ -r "$DIR/cache_guard.py" ]; then
  python3 "$DIR/cache_guard.py" || exit 0
fi

exit 0
