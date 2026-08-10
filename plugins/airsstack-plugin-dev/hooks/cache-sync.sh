#!/bin/sh
# airsstack-plugin-dev PostToolUse cache-sync launcher.
#
# Same rule as the guard's launcher: check the runtime is there, never propagate a child's status,
# and no `exec`. A PostToolUse failure must not disturb the tool call that triggered it.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0
command -v airsl >/dev/null 2>&1 || exit 0

airsl run --fail-open --policy confined \
  --allow-env HOME --allow-env AIRSSTACK_PLUGIN_DEV_DEBUG \
  --allow-read / --allow-write "$HOME/.claude/plugins/cache" \
  "$DIR/cache_sync.lua" || exit 0

exit 0
