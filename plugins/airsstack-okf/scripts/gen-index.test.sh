#!/bin/sh
# Tests for gen-index.sh — exact output, byte-reproducibility, okf_version
# preservation, unparseable-file skip.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

B="$TMP/bundle"
mkdir -p "$B/tables" "$B/metrics"
printf -- '---\nokf_version: "0.1"\n---\n\n# Index\n' > "$B/index.md"
printf -- '## 2026-07-02\n\n- **Creation** — provisioned empty OKF bundle.\n' > "$B/log.md"
cat > "$B/overview.md" <<'EOF'
---
type: Reference
title: Overview
description: What this bundle covers.
---

Body.
EOF
cat > "$B/tables/orders.md" <<'EOF'
---
type: Table
title: Orders
description: One row per order.
---

Body.
EOF
cat > "$B/tables/customers.md" <<'EOF'
---
type: Table
---

Body.
EOF
cat > "$B/metrics/wau.md" <<'EOF'
---
type: Metric
title: Weekly Active Users
description: Distinct users per week.
---

Body.
EOF

cat > "$TMP/expected.md" <<'EOF'
---
okf_version: "0.1"
---

# Index

- [Overview](/overview.md) — What this bundle covers.

## metrics

- [Weekly Active Users](/metrics/wau.md) — Distinct users per week.

## tables

- [customers](/tables/customers.md)
- [Orders](/tables/orders.md) — One row per order.
EOF

# 1. Exact regeneration output.
sh "$SCRIPT_DIR/gen-index.sh" "$B" 2>/dev/null
diff -u "$TMP/expected.md" "$B/index.md" >/dev/null 2>&1
check $? "regenerated index.md matches expected byte-for-byte"

# 2. Byte-reproducible: second run changes nothing.
cp "$B/index.md" "$TMP/first.md"
sh "$SCRIPT_DIR/gen-index.sh" "$B" 2>/dev/null
diff "$TMP/first.md" "$B/index.md" >/dev/null 2>&1
check $? "second run is byte-identical"

# 3. okf_version frontmatter block survives regeneration.
head -n 3 "$B/index.md" | grep -q 'okf_version: "0.1"'
check $? "okf_version block preserved"

# 4. Unparseable frontmatter → skipped with a warning, exit 0.
printf -- '---\ntype: Broken\nno closing fence\n' > "$B/broken.md"
err=$( sh "$SCRIPT_DIR/gen-index.sh" "$B" 2>&1 >/dev/null )
rc=$?
[ $rc -eq 0 ] && printf '%s' "$err" | grep -q 'broken.md' \
  && ! grep -q 'broken.md' "$B/index.md"
check $? "unparseable file skipped with warning (rc=$rc '$err')"

# 5. Usage error → exit 2.
sh "$SCRIPT_DIR/gen-index.sh" "$TMP/no-such-dir" >/dev/null 2>&1
[ $? -eq 2 ]; check $? "missing bundle dir exits 2"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
