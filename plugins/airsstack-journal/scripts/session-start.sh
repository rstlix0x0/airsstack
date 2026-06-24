#!/bin/sh
# SessionStart orchestration for airsstack-journal: provision, then rebuild the
# derived index only when stale or absent. Fail-open: never block the session,
# never invoke a model. No `set -e` — each fallible step is guarded with || exit 0.
set -u

here=$(CDPATH= cd "$(dirname "$0")" && pwd)
root="${AIRSSTACK_HOME:-$HOME/.airsstack}/journal"
marker="$root/.index/summaries.tsv"

# 1. Always provision (cheap).
sh "$here/provision.sh" >/dev/null 2>&1 || exit 0

# 2. Staleness check against the index marker.
needs_build=0
if [ ! -f "$marker" ]; then
  needs_build=1
else
  newer=$(find "$root/daily" "$root/sessions" "$root/notes" "$root/mocs" \
            -name '*.md' -newer "$marker" -print 2>/dev/null | head -n 1)
  [ -n "$newer" ] && needs_build=1
fi

# 3. Conditionally rebuild, failing open (e.g. python3 absent).
if [ "$needs_build" -eq 1 ] && command -v python3 >/dev/null 2>&1; then
  python3 "$here/build-index.py" >/dev/null 2>&1 || exit 0
fi

# 4. Orientation card (best-effort, fail-open): build a project-scoped
#    recent-activity card and inject it as SessionStart additionalContext.
#    JSON-encode with python3; if python3 is absent the card is simply skipped
#    (the index also did not rebuild) — the session is never blocked.
card=$(sh "$here/orientation.sh" 2>/dev/null) || card=""
if [ -n "$card" ] && command -v python3 >/dev/null 2>&1; then
  CARD="$card" python3 -c '
import json, os
ctx = os.environ.get("CARD", "")
print(json.dumps({"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": ctx}}))
' 2>/dev/null || exit 0
fi

exit 0
