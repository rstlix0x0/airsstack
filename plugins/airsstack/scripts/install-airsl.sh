#!/bin/sh
# Installs the airsl runtime that every airsstack plugin hook runs on.
#
# User-invoked, never automatic. It compiles from source, which takes minutes — a session-start
# hook that silently triggered that would be worse than the problem it solves. The SessionStart
# preflight names this script instead of running it.
#
# Pure POSIX sh with no airsl dependency, because the situation it exists for is "airsl is missing".
#
#   sh plugins/airsstack/scripts/install-airsl.sh [--force]

set -u

REPO="https://github.com/rstlix0x0/airsstack"

force=0
if [ "${1:-}" = "--force" ]; then
  force=1
fi

# The resolution order the hook wrappers use, kept identical on purpose: if this script and the
# wrappers disagreed about whether airsl is present, the warning and the remedy would contradict
# each other.
find_airsl() {
  if [ -n "${AIRSL_BIN:-}" ] && [ -x "${AIRSL_BIN:-}" ]; then
    echo "$AIRSL_BIN"
    return 0
  fi
  if command -v airsl >/dev/null 2>&1; then
    command -v airsl
    return 0
  fi
  for candidate in "${CARGO_HOME:-$HOME/.cargo}/bin/airsl" "$HOME/.cargo/bin/airsl"; do
    if [ -x "$candidate" ]; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}

# Whether `directory` is on PATH. The failure this catches is the quiet one: airsl installed to
# ~/.cargo/bin, but hooks spawned by a shell that never sourced a profile cannot see it, so every
# hook no-ops and nothing says why.
on_path() {
  case ":${PATH:-}:" in
    *":$1:"*) return 0 ;;
    *) return 1 ;;
  esac
}

report() {
  found="$1"
  directory=$(dirname "$found")
  echo "airsl: $found"
  "$found" doctor 2>/dev/null || true
  if ! on_path "$directory"; then
    echo
    echo "WARNING: $directory is not on this shell's PATH."
    echo "The binary exists but plugin hooks resolve it by PATH first, so they may still"
    echo "see nothing. Add it to PATH, or set AIRSL_BIN=$found in your environment."
  fi
}

if existing=$(find_airsl) && [ "$force" -eq 0 ]; then
  report "$existing"
  echo
  echo "Already installed. Re-run with --force to rebuild from $REPO."
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. airsl is built from source, so a Rust toolchain is required." >&2
  echo "Install one from https://rustup.rs, then re-run this script." >&2
  exit 1
fi

echo "Installing airsl from $REPO"
echo "(builds ~79 crates from source; expect a few minutes)"
echo

# --locked builds against the committed Cargo.lock. Without it a semver-compatible upstream
# release can break the build on a machine nobody here can see.
if [ "$force" -eq 1 ]; then
  cargo install --git "$REPO" --locked --force airsl-cli || exit 1
else
  cargo install --git "$REPO" --locked airsl-cli || exit 1
fi

echo
if installed=$(find_airsl); then
  report "$installed"
  exit 0
fi

echo "cargo install reported success, but airsl is still not resolvable." >&2
echo "Check that cargo's bin directory exists and is readable." >&2
exit 1
