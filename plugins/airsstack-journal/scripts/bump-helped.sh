#!/bin/sh
# Increment the helped: counter of one journal note, then rebuild the index so
# ranking reflects the new value. Usage: bump-helped.sh <stem>
# Resolves <stem>.md case-insensitively across notes/ sessions/ daily/ mocs/.
# Leaves `updated` alone (a usage bump is not a content edit). Missing stem or
# non-integer helped: → stderr + nonzero (a deliberate user action; surface the
# error rather than fail silent).
set -u

stem_in="${1:-}"
if [ -z "$stem_in" ]; then
  printf 'bump-helped: usage: bump-helped.sh <stem>\n' >&2
  exit 1
fi

here=$(CDPATH= cd "$(dirname "$0")" && pwd)
root="${AIRSSTACK_HOME:-$HOME/.airsstack}/journal"

# Case-insensitive resolve against note stems across all note-bearing dirs.
want=$(printf '%s' "$stem_in" | tr '[:upper:]' '[:lower:]')
target=""
for d in notes sessions daily mocs; do
  for f in "$root"/"$d"/*.md; do
    [ -e "$f" ] || continue
    base=$(basename "$f" .md | tr '[:upper:]' '[:lower:]')
    if [ "$base" = "$want" ]; then target="$f"; break; fi
  done
  [ -n "$target" ] && break
done

if [ -z "$target" ]; then
  printf 'bump-helped: no note matching %s.md under notes/ sessions/ daily/ mocs/\n' "$stem_in" >&2
  exit 1
fi

# Relative path for receipts (e.g. sessions/session-5481f011.md).
rel=${target#"$root"/}

cur=$(awk -F': *' '/^helped:/ { print $2; exit }' "$target")
case "$cur" in
  ''|*[!0-9]*)
    printf 'bump-helped: %s has no integer helped:\n' "$target" >&2
    exit 1 ;;
esac
new=$((cur + 1))

tmp=$(mktemp)
awk -v new="$new" '
  /^helped:/ && !done { print "helped: " new; done=1; next }
  { print }
' "$target" > "$tmp" && mv "$tmp" "$target"

if command -v python3 >/dev/null 2>&1; then
  python3 "$here/build-index.py" --force >/dev/null 2>&1 || true
  printf 'bumped helped to %s in %s\n' "$new" "$rel"
else
  printf 'bumped helped to %s in %s (index rebuild deferred: no python3)\n' "$new" "$rel"
fi
exit 0
