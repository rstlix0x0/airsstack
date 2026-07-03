#!/bin/sh
# Resolve the OKF bundle root. Resolution order:
#   1. explicit path argument (wins; must be an existing directory)
#   2. <repo-root>/knowledge/index.md carrying okf_version: frontmatter
#   3. bounded scan of git-visible index.md files (tracked + untracked,
#      not ignored) for the okf_version marker; no-git falls back to find
# Prints the absolute bundle-root path on stdout, exits 0.
# Exit 2: bad usage / no bundle / ambiguous (message on stderr).
# Paths containing whitespace are not supported (suite-wide limitation).
set -u

# A marker file is an index.md whose leading frontmatter block (bounded
# check: first line ---, a closing --- exists) contains okf_version:.
has_marker() {
  [ -f "$1" ] || return 1
  awk '
    NR==1 { if ($0 != "---") { bad=1; exit }; next }
    $0 == "---" { done=1; exit }
    /^okf_version:[[:space:]]*[^[:space:]]/ { found=1 }
    END { if (bad || !done || !found) exit 1 }
  ' "$1"
}

abs_dir() { CDPATH= cd "$1" 2>/dev/null && pwd -P; }

# 1. Explicit argument wins.
if [ "$#" -ge 1 ] && [ -n "${1:-}" ]; then
  if dir=$(abs_dir "$1"); then
    printf '%s\n' "$dir"
    exit 0
  fi
  printf 'okf-root: explicit path is not a directory: %s\n' "$1" >&2
  exit 2
fi

# 2. Conventional default: <repo-root>/knowledge/.
if top=$(git rev-parse --show-toplevel 2>/dev/null); then
  base=$top
else
  base=$(pwd -P)
fi
if has_marker "$base/knowledge/index.md"; then
  printf '%s\n' "$base/knowledge"
  exit 0
fi

# 3. Bounded scan for marked index.md files.
if git rev-parse --git-dir >/dev/null 2>&1; then
  candidates=$(cd "$base" && git ls-files -co --exclude-standard \
    | grep -E '(^|/)index\.md$' || true)
else
  candidates=$(cd "$base" && find . -name index.md -not -path '*/.*' \
    | sed 's|^\./||')
fi

hits=$(
  for rel in $candidates; do
    if has_marker "$base/$rel"; then dirname "$base/$rel"; fi
  done | LC_ALL=C sort -u
)

count=$(printf '%s' "$hits" | grep -c . || true)
case "$count" in
  1) printf '%s\n' "$hits"; exit 0 ;;
  0) printf 'okf-root: no OKF bundle found — run /airsstack-okf:okf-setup or pass an explicit path\n' >&2
     exit 2 ;;
  *) printf 'okf-root: multiple bundle candidates — pass an explicit path:\n%s\n' "$hits" >&2
     exit 2 ;;
esac
