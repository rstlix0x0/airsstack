#!/bin/sh
# Provision the airsstack-journal vault directories. Idempotent, zero-dependency.
set -eu

root="${AIRSSTACK_HOME:-$HOME/.airsstack}/journal"

for d in daily sessions notes mocs .index; do
  mkdir -p "$root/$d"
done

printf 'journal: vault provisioned at %s\n' "$root"
