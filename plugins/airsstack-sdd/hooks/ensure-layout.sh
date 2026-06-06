#!/usr/bin/env sh
# Single source of truth (shell side) for the SDD artifact tree location.
# The prose mirror is plugins/airsstack-sdd/references/artifact-paths.md; the two
# MUST agree. Change one, change the other.
#
# Idempotent: creates only what is missing; never duplicates the .gitignore line.
# Runs relative to the current working directory (the consuming project root).
set -eu

SDD_ROOT=".airsstack/cc/plugins/sdd"

created=""

for d in "$SDD_ROOT/rfcs" "$SDD_ROOT/specs" "$SDD_ROOT/plans" "$SDD_ROOT/plans/_archive"; do
  if [ ! -d "$d" ]; then
    mkdir -p "$d"
    created="${created}  created ${d}
"
  fi
done

# Ensure .gitignore ignores the whole .airsstack/ tree, exactly once.
if [ ! -f .gitignore ]; then
  printf '.airsstack/\n' > .gitignore
  created="${created}  created .gitignore with .airsstack/
"
elif ! grep -qxF '.airsstack/' .gitignore; then
  printf '.airsstack/\n' >> .gitignore
  created="${created}  appended .airsstack/ to .gitignore
"
fi

if [ -n "$created" ]; then
  printf 'airsstack-sdd layout provisioned:\n'
  printf '%s' "$created"
else
  printf 'airsstack-sdd layout already present; nothing to do.\n'
fi
