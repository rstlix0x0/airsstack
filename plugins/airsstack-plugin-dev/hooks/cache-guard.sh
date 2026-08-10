#!/bin/sh
# airsstack-plugin-dev cache delivery guard launcher.
#
# `airsl` is not ambient the way `sh` was: until the binary is installed it is not there, and a
# launcher that checks first keeps a missing runtime silent rather than broken. Never propagate a
# child's status — deliberately no `exec`, which would hand airsl's status back to Claude Code.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0
command -v airsl >/dev/null 2>&1 || exit 0

airsl run --fail-open --policy confined \
  --allow-env HOME \
  --allow-read / --allow-write "$HOME/.claude/plugins/cache" \
  --allow-exec git \
  "$DIR/cache_guard.lua" || exit 0

exit 0
