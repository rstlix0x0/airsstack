#!/bin/sh
# Tests for cmux-snap — stubs `cmux`, asserts snapshot→action→snapshot-after order.
set -u
fail=0
SCRIPT_DIR=$(CDPATH='' cd "$(dirname "$0")" && pwd)
SCRIPT="$SCRIPT_DIR/cmux-snap"
check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"
LOG="$TMP/calls.log"
cat > "$TMP/bin/cmux" <<EOF
#!/bin/sh
echo "\$*" >> "$LOG"
exit 0
EOF
chmod +x "$TMP/bin/cmux"

PATH="$TMP/bin:$PATH" sh "$SCRIPT" surface:2 click "button.submit" >/dev/null
check $? "runs to completion"

# Line 1: pre-snapshot. Line 2: action with --snapshot-after.
l1=$(sed -n 1p "$LOG"); l2=$(sed -n 2p "$LOG")
printf '%s' "$l1" | grep -q 'browser surface:2 snapshot --interactive'; check $? "pre-snapshot first ($l1)"
printf '%s' "$l2" | grep -q 'browser surface:2 click button.submit --snapshot-after'; check $? "action with --snapshot-after ($l2)"

# Requires surface + action.
PATH="$TMP/bin:$PATH" sh "$SCRIPT" surface:2 >/dev/null 2>&1 && _r=1 || _r=0
check "$_r" "requires an action"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
