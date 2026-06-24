#!/bin/sh
# Print a project-scoped recent-activity orientation card from summaries.tsv.
# Usage: orientation.sh [project]   (project defaults to project-key.sh output)
# Pure POSIX sh + awk: no python, no model. Fail-open: any failure (no tsv, no
# match, empty vault) prints nothing and exits 0, so it never blocks a session.
set -u

here=$(CDPATH= cd "$(dirname "$0")" && pwd)
root="${AIRSSTACK_HOME:-$HOME/.airsstack}/journal"
tsv="$root/.index/summaries.tsv"

[ -f "$tsv" ] || exit 0

proj="${1:-}"
if [ -z "$proj" ]; then
  proj=$(sh "$here/project-key.sh" 2>/dev/null) || exit 0
fi
[ -n "$proj" ] || exit 0

# Emit "updated<TAB>stem<TAB>summary" rows for one kind, newest first, capped.
# kind=session → stems matching ^session-; kind=note → all others.
emit() {
  awk -F'\t' -v proj="$proj" -v kind="$1" '
    function member(col, p,   n, a, i) {
      n = split(col, a, ", ")
      for (i = 1; i <= n; i++) if (a[i] == p) return 1
      return 0
    }
    member($4, proj) {
      is_sess = ($1 ~ /^session-/)
      if ((kind == "session" && is_sess) || (kind == "note" && !is_sess))
        printf "%s\t%s\t%s\n", $6, $1, $3
    }
  ' "$tsv" | sort -r | head -n "$2"
}

sessions=$(emit session 3)
notes=$(emit note 5)
[ -z "$sessions" ] && [ -z "$notes" ] && exit 0

printf '## Journal — recent activity (%s)\n' "$proj"
if [ -n "$sessions" ]; then
  printf '\n**Sessions:**\n'
  printf '%s\n' "$sessions" | while IFS='	' read -r upd stem summ; do
    printf -- '- [[%s]] — %s\n' "$stem" "$summ"
  done
fi
if [ -n "$notes" ]; then
  printf '\n**Notes:**\n'
  printf '%s\n' "$notes" | while IFS='	' read -r upd stem summ; do
    printf -- '- [[%s]] — %s\n' "$stem" "$summ"
  done
fi
exit 0
