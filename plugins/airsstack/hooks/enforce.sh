#!/bin/sh
# airsstack rule-enforcement dispatcher launcher.
#
# PreToolUse exit 2 BLOCKS the tool call, and the matcher covers Read — a propagated failure would
# block every file read. This launcher therefore never propagates a child's status: every path ends
# in `exit 0`. Deliberately no `exec`, which would replace the shell and hand airsl's status
# straight back to Claude Code.
#
# The read grant is `/` because the dispatcher is handed an arbitrary file_path by the tool call it
# is inspecting, and must stat it, its ancestors, and the install cache. What the confinement still
# buys over `--policy trusted` is real: no `io`, no `os.execute`, writes only under the sentinel
# directory, and `git` as the only program it can run.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0
command -v airsl >/dev/null 2>&1 || exit 0

SENTINELS="${TMPDIR:-/tmp}"

airsl run --fail-open --policy confined \
  --allow-env HOME --allow-env TMPDIR \
  --allow-env AIRSSTACK_HOME --allow-env AIRSSTACK_ENFORCE_REGISTRY \
  --allow-read / --allow-write "$SENTINELS" \
  --allow-exec git \
  "$DIR/enforce.lua" || exit 0

exit 0
