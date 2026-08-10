#!/bin/sh
# airsstack-sdd SessionStart launcher.
#
# `airsl` is not ambient the way `sh` was: until the binary is installed it is not there, and a
# launcher that checks first keeps a missing runtime silent rather than broken. Every path exits 0,
# and deliberately no `exec` — that would replace the shell and hand airsl's status back to Claude
# Code.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0
command -v airsl >/dev/null 2>&1 || exit 0

HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"

airsl run --fail-open --policy confined \
  --allow-env AIRSSTACK_HOME --allow-env HOME \
  --allow-read . --allow-write . \
  --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" \
  --allow-exec git \
  "$DIR/ensure-layout.lua" || exit 0

exit 0
