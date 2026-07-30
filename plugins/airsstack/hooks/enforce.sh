#!/bin/sh
# airsstack rule-enforcement dispatcher launcher.
#
# PreToolUse exit 2 BLOCKS the tool call, and the matcher covers Read — a
# propagated failure would block every file read. This launcher therefore
# never propagates a child's status: every path ends in `exit 0`.
# Deliberately no `exec`, which would replace the shell and hand python's
# status straight back to Claude Code.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0

if command -v python3 >/dev/null 2>&1 && [ -r "$DIR/enforce.py" ]; then
  python3 "$DIR/enforce.py" || exit 0
fi

exit 0
