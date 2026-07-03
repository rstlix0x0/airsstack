#!/bin/sh
# OKF v0.1 conformance lint. Usage: okf-lint.sh <bundle-root>
#
# HARD BAR (any hit → exit 1), exactly the spec's conformance rules:
#   - every non-reserved .md: leading --- fence, closing --- fence, and a
#     non-empty type: inside the block (bounded structural check — full
#     YAML validation is deliberately out of scope)
#   - log.md: no frontmatter; every '## ' heading is ISO YYYY-MM-DD; every
#     other non-blank line is a '# ' title, a '- ' list item, or an
#     indented continuation
#   - index.md: no frontmatter — EXCEPT the bundle-root index.md, whose
#     block may contain only okf_version lines
#
# WARNINGS (never affect the exit code — permissive consumption):
#   - broken bundle-relative links: [text](/path.md) with no such file
#     (the recommended absolute link form is the only form checked)
#   - missing recommended fields: title, description, timestamp
#
# Exit: 0 conformant (warnings allowed) · 1 hard failure · 2 usage.
# Paths containing whitespace are not supported (suite-wide limitation).
set -u

if [ "$#" -ne 1 ] || [ ! -d "${1:-}" ]; then
  printf 'usage: okf-lint.sh <bundle-root>\n' >&2
  exit 2
fi
root=$(CDPATH= cd "$1" && pwd -P)
fails=0
warns=0

fail() { printf 'FAIL: %s: %s\n' "$1" "$2"; fails=$((fails + 1)); }
warn() { printf 'WARN: %s: %s\n' "$1" "$2"; warns=$((warns + 1)); }

files=$(cd "$root" && find . -name '*.md' -not -path '*/.*' \
  | sed 's|^\./||' | LC_ALL=C sort)

for rel in $files; do
  file="$root/$rel"
  case "$(basename "$rel")" in
    index.md)
      if head -n 1 "$file" | grep -qx -- '---'; then
        if [ "$rel" = "index.md" ]; then
          bad=$(awk 'NR==1 { next } $0=="---" { exit }
                     $0 !~ /^okf_version:/ { print; exit }' "$file")
          [ -z "$bad" ] || fail "$rel" "root index.md frontmatter may contain only okf_version (found: $bad)"
        else
          fail "$rel" "index.md must not carry frontmatter"
        fi
      fi
      ;;
    log.md)
      if head -n 1 "$file" | grep -qx -- '---'; then
        fail "$rel" "log.md must not carry frontmatter"
      fi
      badhead=$(grep -n '^## ' "$file" \
        | grep -v -E '^[0-9]+:## [0-9]{4}-[0-9]{2}-[0-9]{2}$' \
        | head -n 1 || true)
      [ -z "$badhead" ] || fail "$rel" "non-ISO date heading: ${badhead#*:}"
      badline=$(awk '/^$/ { next }
                     /^## / { next }
                     /^# / { next }
                     /^- / { next }
                     /^[[:space:]]/ { next }
                     { print; exit }' "$file")
      [ -z "$badline" ] || fail "$rel" "log entry is not a list item: $badline"
      ;;
    *)
      if ! awk 'NR==1 { if ($0 != "---") { bad=1; exit }; next }
                $0=="---" { done=1; exit }
                END { if (bad || !done) exit 1 }' "$file"; then
        fail "$rel" "missing or unclosed frontmatter fence"
        continue
      fi
      typeval=$(awk 'NR==1 { next } $0=="---" { exit }
                     index($0, "type:") == 1 {
                       val = substr($0, 6)
                       sub(/^[[:space:]]*/, "", val)
                       sub(/^"/, "", val); sub(/"$/, "", val)
                       print val; exit
                     }' "$file")
      [ -n "$typeval" ] || fail "$rel" "missing or empty required field: type"
      for key in title description timestamp; do
        got=$(awk -v key="$key" 'NR==1 { next } $0=="---" { exit }
                                 index($0, key ":") == 1 { print "y"; exit }' "$file")
        [ -n "$got" ] || warn "$rel" "missing recommended field: $key"
      done
      links=$(grep -o '](/[^)]*\.md)' "$file" \
        | sed 's|^](/||; s|)$||' | LC_ALL=C sort -u || true)
      for l in $links; do
        [ -f "$root/$l" ] || warn "$rel" "broken link: /$l"
      done
      ;;
  esac
done

printf 'okf-lint: %d failure(s), %d warning(s)\n' "$fails" "$warns"
[ "$fails" -eq 0 ] || exit 1
exit 0
