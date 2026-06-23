#!/bin/sh
# Idempotently link a note stem into a day's daily structure note.
# Usage: daily-link.sh <YYYY-MM-DD> <stem>
# Creates daily/<date>.md (frontmatter + "## Notes" list) when absent; appends
# "- [[<stem>]]" only when the link is not already present; bumps `updated`.
# Honours AIRSSTACK_HOME. Always exits 0.
set -u

date="$1"
stem="$2"
root="${AIRSSTACK_HOME:-$HOME/.airsstack}/journal"
file="$root/daily/$date.md"
now=$(date '+%Y-%m-%d %H:%M')

mkdir -p "$root/daily"

if [ ! -f "$file" ]; then
  cat > "$file" <<EOF
---
title: $date
type: daily
created: $now
updated: $now
helped: 0
---

## Notes
EOF
fi

# Idempotent: do nothing if the link is already present.
if grep -qF "[[$stem]]" "$file" 2>/dev/null; then
  exit 0
fi

printf -- '- [[%s]]\n' "$stem" >> "$file"

# Bump `updated` in place (portable rewrite via a temp file).
tmp=$(mktemp)
awk -v now="$now" '
  /^updated:/ && !bumped { print "updated: " now; bumped=1; next }
  { print }
' "$file" > "$tmp" && mv "$tmp" "$file"

exit 0
