#!/bin/sh
# Tests for cmux-settings — backup→edit→validate with revert-on-invalid.
set -u
fail=0
SCRIPT_DIR=$(CDPATH='' cd "$(dirname "$0")" && pwd)
SCRIPT="$SCRIPT_DIR/cmux-settings"
check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"
cfg="$TMP/cmux.json"
printf '{"a":1}\n' > "$cfg"

# Stub cmux: `config validate` exits per VALIDATE_RC env in the stub dir.
cat > "$TMP/bin/cmux" <<EOF
#!/bin/sh
if [ "\$1" = "config" ] && [ "\$2" = "validate" ]; then
  exit "\${VALIDATE_RC:-0}"
fi
exit 0
EOF
chmod +x "$TMP/bin/cmux"

# 1. backup creates a timestamped .bak next to the config.
PATH="$TMP/bin:$PATH" CMUX_SETTINGS_FILE="$cfg" sh "$SCRIPT" backup >/dev/null
ls "$TMP"/cmux.json.*.bak >/dev/null 2>&1; check $? "backup writes a timestamped .bak"

# 2. backup-then with a valid result keeps the edit.
PATH="$TMP/bin:$PATH" CMUX_SETTINGS_FILE="$cfg" VALIDATE_RC=0 \
  sh "$SCRIPT" backup-then sh -c "printf '{\"a\":2}\n' > \"$cfg\"" >/dev/null
grep -q '"a":2' "$cfg"; check $? "valid edit is kept"

# 3. backup-then with an invalid result reverts the edit and exits nonzero.
printf '{"a":2}\n' > "$cfg"
PATH="$TMP/bin:$PATH" CMUX_SETTINGS_FILE="$cfg" VALIDATE_RC=1 \
  sh "$SCRIPT" backup-then sh -c "printf 'BROKEN' > \"$cfg\"" >/dev/null 2>&1
rc=$?
{ [ "$rc" -ne 0 ] && grep -q '"a":2' "$cfg"; }
check $? "invalid edit reverts + nonzero (rc=$rc, cfg=$(cat "$cfg"))"

# 4. missing config for backup → nonzero.
! PATH="$TMP/bin:$PATH" CMUX_SETTINGS_FILE="$TMP/absent.json" sh "$SCRIPT" backup >/dev/null 2>&1
check $? "backup of absent config → nonzero"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
