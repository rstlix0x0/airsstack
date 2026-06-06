#!/bin/sh
# airsstack concise hook launcher.
#
# Prefer python3; fall back to node. Forward stdin to whichever runtime is
# present so the hook works on machines with either. If neither exists, exit
# silently (0) so the user prompt is never blocked.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

if command -v python3 >/dev/null 2>&1; then
  exec python3 "$DIR/concise-tracker.py"
elif command -v node >/dev/null 2>&1; then
  exec node "$DIR/concise-tracker.js"
fi

exit 0
