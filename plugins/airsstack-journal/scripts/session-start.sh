#!/bin/sh
# airsstack-journal SessionStart launcher.
#
# The only shell left in this plugin, and it exists for one reason: `airsl` is not ambient the way
# `sh` was. Until the binary is installed it is simply not there, and a launcher that checks first
# keeps a missing runtime silent rather than broken.
#
# Every path exits 0. Deliberately no `exec`, which would replace the shell and hand airsl's exit
# status straight back to Claude Code.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0
command -v airsl >/dev/null 2>&1 || exit 0

HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"

airsl run --fail-open --policy confined \
  --allow-env AIRSSTACK_HOME --allow-env HOME \
  --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" \
  --allow-read . --allow-exec git \
  "$DIR/session-start.lua" || exit 0

exit 0
