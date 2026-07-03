#!/bin/sh
# Tests for okf-root.sh — explicit arg wins; knowledge/ convention; bounded
# scan; zero and multiple candidates exit 2.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

mkbundle() { # $1 = dir — create a marked OKF bundle root
  mkdir -p "$1"
  printf -- '---\nokf_version: "0.1"\n---\n\n# Index\n' > "$1/index.md"
}

# 1. Explicit path argument wins (printed absolute), even without a marker.
mkdir -p "$TMP/explicit/anydir"
out=$( cd "$TMP/explicit" && sh "$SCRIPT_DIR/okf-root.sh" anydir )
[ "$out" = "$(CDPATH= cd "$TMP/explicit/anydir" && pwd -P)" ]
check $? "explicit arg wins, absolute path (got '$out')"

# 2. Explicit path that is not a directory → exit 2.
( cd "$TMP/explicit" && sh "$SCRIPT_DIR/okf-root.sh" no-such-dir >/dev/null 2>&1 )
[ $? -eq 2 ]; check $? "nonexistent explicit path exits 2"

# 3. knowledge/ with marker in a git repo → <top>/knowledge, from a subdir too.
mkdir -p "$TMP/repo1/src"
( cd "$TMP/repo1" && git init -q )
mkbundle "$TMP/repo1/knowledge"
out=$( cd "$TMP/repo1/src" && sh "$SCRIPT_DIR/okf-root.sh" )
[ "$out" = "$(CDPATH= cd "$TMP/repo1/knowledge" && pwd -P)" ]
check $? "knowledge/ convention resolves from subdir (got '$out')"

# 4. No bundle anywhere → exit 2, message names okf-setup.
mkdir -p "$TMP/repo2"
( cd "$TMP/repo2" && git init -q )
err=$( cd "$TMP/repo2" && sh "$SCRIPT_DIR/okf-root.sh" 2>&1 >/dev/null )
rc=$?
[ $rc -eq 2 ] && printf '%s' "$err" | grep -q 'okf-setup'
check $? "no bundle exits 2 and mentions okf-setup (got rc=$rc '$err')"

# 5. Single non-default bundle (untracked) found by the scan.
mkdir -p "$TMP/repo3"
( cd "$TMP/repo3" && git init -q )
mkbundle "$TMP/repo3/docs/kb"
out=$( cd "$TMP/repo3" && sh "$SCRIPT_DIR/okf-root.sh" )
[ "$out" = "$(CDPATH= cd "$TMP/repo3/docs/kb" && pwd -P)" ]
check $? "scan finds single non-default bundle (got '$out')"

# 6. Multiple candidates → exit 2, both listed.
mkbundle "$TMP/repo3/other/kb"
err=$( cd "$TMP/repo3" && sh "$SCRIPT_DIR/okf-root.sh" 2>&1 >/dev/null )
rc=$?
[ $rc -eq 2 ] && printf '%s' "$err" | grep -q 'docs/kb' && printf '%s' "$err" | grep -q 'other/kb'
check $? "multiple candidates exit 2 and are listed (got rc=$rc)"

# 7. index.md WITHOUT okf_version is not a marker.
mkdir -p "$TMP/repo4/notes"
( cd "$TMP/repo4" && git init -q )
printf '# just an index\n' > "$TMP/repo4/notes/index.md"
( cd "$TMP/repo4" && sh "$SCRIPT_DIR/okf-root.sh" >/dev/null 2>&1 )
[ $? -eq 2 ]; check $? "plain index.md without okf_version is ignored"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
