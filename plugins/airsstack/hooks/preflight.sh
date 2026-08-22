#!/bin/sh
# airsstack SessionStart preflight.
#
# Two jobs, and the second is why this is a script rather than the `echo` it replaced: report when
# the airsl runtime cannot be found. A SessionStart command hook's bare stdout is not printed to the
# user's terminal — it is injected into the model's context (crates/claudevs/src/harness/semantics.rs
# maps it that way, and asserts it in a test). So this script does not reach the user directly; it
# reaches the model as context, not as an instruction, and the warning below is written that way: it
# states what is broken, what is disabled, and how to fix it, with nothing aimed at the model — hook
# stdout is model-visible context, not a place to steer the model's next reply. Every other hook in
# the suite is guarded to exit silently without airsl, so a machine that lacks it loses rule
# enforcement, the concise tracker, SDD layout provisioning and the journal orientation card — all
# with no signal anywhere. This is still the right place for the check, because it needs nothing but
# POSIX sh and so works precisely on the machines where every airsl-backed hook is dead.
#
# Always exits 0; a preflight that broke a session start would be worse than the gap it reports.

echo 'Reminder: run /airsstack:snapshot-load for the current git branch to rehydrate relevant project memory before starting work.'

# The resolution order the hook wrappers and scripts/install-airsl.sh use, kept identical: a
# preflight that disagreed with the wrappers would announce a problem they do not have, or stay
# quiet about one they do.
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

if [ -z "$AIRSL" ]; then
  DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || DIR=""
  echo
  echo 'STATUS: the airsl runtime was not found, so every airsstack plugin hook is inert.'
  echo 'Disabled: rule enforcement, the concise tracker, SDD layout provisioning, and the'
  echo 'journal orientation card.'
  echo 'FIX: install airsl, then start a new session.'
  if [ -n "$DIR" ]; then
    echo "  sh \"$DIR/../scripts/install-airsl.sh\""
  else
    echo '  cargo install --git https://github.com/rstlix0x0/airsstack --locked airsl-cli'
  fi
fi

exit 0
