#!/bin/sh
# Deny writes touching a lockfile; emit a defer envelope for .rs files; stay
# silent otherwise. Crashes (exit 1, no output) on non-JSON stdin — the
# hostile-input behavior the crash case pins.
payload=$(cat)
case "$payload" in
  "{"*"}") ;;
  *) exit 1 ;;
esac
case "$payload" in
  *Cargo.lock*|*poetry.lock*) echo 'blocked: lockfile' >&2; exit 2 ;;
  *.rs*) printf '%s' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"defer","additionalContext":"rust-guidelines apply"}}' ;;
  *) exit 0 ;;
esac
