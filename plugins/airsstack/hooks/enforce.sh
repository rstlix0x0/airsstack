#!/bin/sh
# airsstack rule-enforcement dispatcher launcher.
#
# Prefer python3; fall back to node. Forward stdin to whichever runtime is
# present. If neither exists, exit 0 so the edit is never blocked (fail-open).

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

if command -v python3 >/dev/null 2>&1; then
  exec python3 "$DIR/enforce.py"
elif command -v node >/dev/null 2>&1; then
  exec node "$DIR/enforce.js"
fi

exit 0
