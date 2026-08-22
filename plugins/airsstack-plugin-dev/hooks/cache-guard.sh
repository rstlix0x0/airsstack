#!/bin/sh
# airsstack-plugin-dev cache delivery guard launcher.
#
# `airsl` is not ambient the way `sh` was: until the binary is installed it is not there, and a
# launcher that checks first keeps a missing runtime silent rather than broken. Never propagate a
# child's status — deliberately no `exec`, which would hand airsl's status back to Claude Code.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0
# Resolve airsl without relying on PATH. Hooks are spawned by the CLI rather than a login shell,
# so a profile may never have been sourced and a cargo-installed binary under ~/.cargo/bin can be
# present but invisible — which reads as "airsl is not installed" and silently disables the hook.
AIRSL=""
if [ -n "${AIRSL_BIN:-}" ] && [ -x "${AIRSL_BIN:-}" ]; then
  AIRSL="$AIRSL_BIN"
elif command -v airsl >/dev/null 2>&1; then
  AIRSL=airsl
else
  for candidate in "${CARGO_HOME:-$HOME/.cargo}/bin/airsl" "$HOME/.cargo/bin/airsl"; do
    if [ -x "$candidate" ]; then
      AIRSL="$candidate"
      break
    fi
  done
fi
[ -n "$AIRSL" ] || exit 0

"$AIRSL" run --fail-open --policy confined \
  --allow-env HOME \
  --allow-read / --allow-write "$HOME/.claude/plugins/cache" \
  --allow-exec git \
  "$DIR/cache_guard.lua" || exit 0

exit 0
