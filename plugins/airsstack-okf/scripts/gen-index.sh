#!/bin/sh
# Regenerate the bundle-root index.md of an OKF bundle, deterministically.
# Usage: gen-index.sh <bundle-root>
#   - preserves the existing leading frontmatter block (the okf_version
#     marker) verbatim when it is well-formed
#   - emits '# Index', then root-level concepts (no heading), then one
#     '## <subdir>' section per top-level subdirectory, entries sorted by
#     path (LC_ALL=C); nested subdirs are listed inside their top section
#   - entry: '- [title](/path.md) — description'; title falls back to the
#     filename stem; a missing description renders the bare link
#   - reserved files (index.md, log.md at any level) are never listed
#   - files with unparseable frontmatter are skipped with a stderr warning
# Output is byte-reproducible: two consecutive runs are identical.
# Exit 0 on success, 2 on usage error.
# Paths containing whitespace are not supported (suite-wide limitation).
set -u

if [ "$#" -ne 1 ] || [ ! -d "${1:-}" ]; then
  printf 'usage: gen-index.sh <bundle-root>\n' >&2
  exit 2
fi
root=$(CDPATH= cd "$1" && pwd -P)
out="$root/index.md.tmp"

# Bounded structural check: first line ---, a closing --- exists.
parseable() {
  awk 'NR==1 { if ($0 != "---") { bad=1; exit }; next }
       $0=="---" { done=1; exit }
       END { if (bad || !done) exit 1 }' "$1"
}

# First frontmatter value for a key, surrounding quotes stripped.
fm_get() { # $1=file $2=key
  awk -v key="$2" '
    NR==1 { next }
    $0=="---" { exit }
    index($0, key ":") == 1 {
      val = substr($0, length(key) + 2)
      sub(/^[[:space:]]*/, "", val)
      sub(/^"/, "", val); sub(/"$/, "", val)
      print val
      exit
    }' "$1"
}

emit_entry() { # $1 = bundle-relative path
  file="$root/$1"
  if ! parseable "$file"; then
    printf 'gen-index: skipping unparseable frontmatter: %s\n' "$1" >&2
    return 0
  fi
  title=$(fm_get "$file" title)
  [ -n "$title" ] || title=$(basename "$1" .md)
  desc=$(fm_get "$file" description)
  if [ -n "$desc" ]; then
    printf -- '- [%s](/%s) — %s\n' "$title" "$1" "$desc" >>"$out"
  else
    printf -- '- [%s](/%s)\n' "$title" "$1" >>"$out"
  fi
}

: >"$out"

# Preserve a well-formed existing leading frontmatter block verbatim.
if [ -f "$root/index.md" ] \
   && head -n 1 "$root/index.md" | grep -qx -- '---' \
   && [ "$(grep -cx -- '---' "$root/index.md")" -ge 2 ]; then
  awk 'NR==1 { print; next } { print } NR>1 && $0=="---" { exit }' \
    "$root/index.md" >>"$out"
  printf '\n' >>"$out"
fi

printf '# Index\n' >>"$out"

concepts=$(cd "$root" && find . -name '*.md' -not -path '*/.*' \
  | sed 's|^\./||' \
  | grep -v -E '(^|/)(index|log)\.md$' \
  | grep -v -E '\.tmp$' \
  | LC_ALL=C sort)

# Root-level concepts first, under no heading.
first_root=1
for rel in $concepts; do
  case "$rel" in */*) continue ;; esac
  if [ "$first_root" -eq 1 ]; then printf '\n' >>"$out"; first_root=0; fi
  emit_entry "$rel"
done

# One section per top-level subdirectory, sorted.
dirs=$(printf '%s\n' "$concepts" | awk -F/ 'NF>1 { print $1 }' | LC_ALL=C sort -u)
for d in $dirs; do
  printf '\n## %s\n\n' "$d" >>"$out"
  for rel in $concepts; do
    case "$rel" in "$d"/*) emit_entry "$rel" ;; esac
  done
done

mv "$out" "$root/index.md"
