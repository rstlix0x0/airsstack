#!/bin/sh
# airsstack-plugin-dev PostToolUse cache-sync launcher.
#
# Same rule as the guard's launcher: check the runtime is there, never propagate a child's status,
# and no `exec`. A PostToolUse failure must not disturb the tool call that triggered it.

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
  --allow-env HOME --allow-env AIRSSTACK_PLUGIN_DEV_DEBUG \
  --allow-read / --allow-write "$HOME/.claude/plugins/cache" \
  "$DIR/cache_sync.lua" || exit 0

exit 0
