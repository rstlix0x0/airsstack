#!/bin/sh
# airsstack enforcement re-arm launcher.
#
# Same rule as enforce.sh: never propagate a child's status, always exit 0. Deliberately no `exec`,
# which would replace the shell and hand airsl's status straight back to Claude Code.

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

SENTINELS="${TMPDIR:-/tmp}"

"$AIRSL" run --fail-open --policy confined \
  --allow-env TMPDIR \
  --allow-read "$SENTINELS" --allow-write "$SENTINELS" \
  "$DIR/rearm.lua" || exit 0

exit 0
