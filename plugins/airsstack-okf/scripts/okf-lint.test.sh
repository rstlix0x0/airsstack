#!/bin/sh
# Tests for okf-lint.sh — hard bar fails exit 1; soft findings warn on
# exit 0; conformant bundle passes clean.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

# --- Fixture 1: conformant bundle -----------------------------------------
C="$TMP/conformant"
mkdir -p "$C/tables"
printf -- '---\nokf_version: "0.1"\n---\n\n# Index\n' > "$C/index.md"
printf -- '## 2026-07-02\n\n- **Creation** — provisioned.\n' > "$C/log.md"
cat > "$C/tables/orders.md" <<'EOF'
---
type: Table
title: Orders
description: One row per order.
timestamp: 2026-07-02T10:00:00Z
---

# Schema

Joined with [customers](/tables/customers.md).
EOF
cat > "$C/tables/customers.md" <<'EOF'
---
type: Table
title: Customers
description: One row per customer.
timestamp: 2026-07-02T10:00:00Z
---

Body.
EOF

out=$( sh "$SCRIPT_DIR/okf-lint.sh" "$C" )
rc=$?
[ $rc -eq 0 ] && printf '%s' "$out" | grep -q '0 failure(s), 0 warning(s)'
check $? "conformant bundle passes clean (rc=$rc)"

# --- Fixture 2: hard-broken bundle -----------------------------------------
H="$TMP/hard"
mkdir -p "$H/sub"
printf -- '---\nokf_version: "0.1"\ntitle: sneaky\n---\n\n# Index\n' > "$H/index.md"
printf -- '## June 5\n\n- **Update** — bad heading.\n' > "$H/log.md"
printf -- '---\ntitle: No Type\n---\n\nBody.\n' > "$H/no-type.md"
printf -- '---\ntype: Broken\nnever closed\n' > "$H/unclosed.md"
printf -- '---\ntype: Meta\n---\n\n# sub index\n' > "$H/sub/index.md"

out=$( sh "$SCRIPT_DIR/okf-lint.sh" "$H" )
rc=$?
[ $rc -eq 1 ] \
  && printf '%s' "$out" | grep -q 'FAIL: no-type.md' \
  && printf '%s' "$out" | grep -q 'FAIL: unclosed.md' \
  && printf '%s' "$out" | grep -q 'FAIL: sub/index.md' \
  && printf '%s' "$out" | grep -q 'FAIL: log.md' \
  && printf '%s' "$out" | grep -q 'FAIL: index.md'
check $? "hard-broken bundle exits 1 with all five failures (rc=$rc)"

# --- Fixture 3: soft-degraded bundle ----------------------------------------
S="$TMP/soft"
mkdir -p "$S"
printf -- '---\nokf_version: "0.1"\n---\n\n# Index\n' > "$S/index.md"
cat > "$S/thin.md" <<'EOF'
---
type: Note
---

See [missing](/nowhere.md).
EOF

out=$( sh "$SCRIPT_DIR/okf-lint.sh" "$S" )
rc=$?
[ $rc -eq 0 ] \
  && printf '%s' "$out" | grep -q 'WARN: thin.md: broken link: /nowhere.md' \
  && printf '%s' "$out" | grep -q 'WARN: thin.md: missing recommended field: title' \
  && printf '%s' "$out" | grep -q 'WARN: thin.md: missing recommended field: description' \
  && printf '%s' "$out" | grep -q 'WARN: thin.md: missing recommended field: timestamp' \
  && printf '%s' "$out" | grep -q '0 failure(s), 4 warning(s)'
check $? "soft-degraded bundle warns but exits 0 (rc=$rc)"

# --- Usage -------------------------------------------------------------------
sh "$SCRIPT_DIR/okf-lint.sh" "$TMP/no-such-dir" >/dev/null 2>&1
[ $? -eq 2 ]; check $? "missing bundle dir exits 2"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
