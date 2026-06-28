#!/bin/sh
# Tests for cmux-layout — stubs `cmux`, asserts the emitted command sequence.
set -u
fail=0
SCRIPT_DIR=$(CDPATH='' cd "$(dirname "$0")" && pwd)
SCRIPT="$SCRIPT_DIR/cmux-layout"
check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"
LOG="$TMP/calls.log"

# Stub cmux: log every invocation; emit a fake ref for new-workspace/new-split.
cat > "$TMP/bin/cmux" <<EOF
#!/bin/sh
echo "\$*" >> "$LOG"
case "\$1" in
  new-workspace) echo "workspace:1" ;;
  new-split) echo "surface:2" ;;
  *) : ;;
esac
exit 0
EOF
chmod +x "$TMP/bin/cmux"

# Two splits, three commands across the three panes.
PATH="$TMP/bin:$PATH" sh "$SCRIPT" --name build --split right --split down \
  --cmd "echo a" --cmd "echo b" --cmd "echo c" >/dev/null
check $? "runs to completion"

grep -q 'new-workspace .*--name build' "$LOG"; check $? "creates named workspace"
if [ "$(grep -c '^new-split right' "$LOG")" -eq 1 ]; then rc=0; else rc=1; fi; check $rc "first split right"
if [ "$(grep -c '^new-split down' "$LOG")" -eq 1 ]; then rc=0; else rc=1; fi; check $rc "second split down"
if [ "$(grep -c '^send ' "$LOG")" -eq 3 ]; then rc=0; else rc=1; fi; check $rc "three commands sent (one per pane)"
grep -q 'send .*echo a' "$LOG"; check $? "cmd a sent"
grep -q 'send .*echo c' "$LOG"; check $? "cmd c sent"

# Reject agent-session crossing of the parked boundary.
! PATH="$TMP/bin:$PATH" sh "$SCRIPT" --name x --provider claude >/dev/null 2>&1
check $? "rejects --provider (no agent spawning)"

# Require --name.
! PATH="$TMP/bin:$PATH" sh "$SCRIPT" --split right >/dev/null 2>&1
check $? "requires --name"

# Failure: new-split exits nonzero → script must exit nonzero.
cat > "$TMP/bin/cmux" <<EOF
#!/bin/sh
echo "\$*" >> "$LOG"
case "\$1" in
  new-workspace) echo "workspace:1" ;;
  new-split) exit 1 ;;
  *) : ;;
esac
exit 0
EOF
chmod +x "$TMP/bin/cmux"
! PATH="$TMP/bin:$PATH" sh "$SCRIPT" --name build --split right >/dev/null 2>&1
check $? "new-split failure propagates nonzero exit"

# Failure: send exits nonzero → script must exit nonzero.
cat > "$TMP/bin/cmux" <<EOF
#!/bin/sh
echo "\$*" >> "$LOG"
case "\$1" in
  new-workspace) echo "workspace:1" ;;
  new-split) echo "surface:2" ;;
  send) exit 1 ;;
  *) : ;;
esac
exit 0
EOF
chmod +x "$TMP/bin/cmux"
! PATH="$TMP/bin:$PATH" sh "$SCRIPT" --name build --split right --cmd "echo a" >/dev/null 2>&1
check $? "send failure propagates nonzero exit"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
