---
name: okf-lint
description: >
  Check the repo's OKF bundle against the v0.1 conformance bar with the
  deterministic lint script and present hard failures separately from
  permissive warnings. Use when the user says "lint the knowledge bundle"
  / "check bundle conformance" / "/okf-lint".
---

# okf-lint

The conformance gate is mechanical — the script decides, you present.

## Steps

1. Resolve the bundle root:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/okf-root.sh" [explicit-path]
   ```

   Exit 2 → relay stderr and STOP.

2. Run the lint:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/okf-lint.sh" "<root>"
   ```

3. Present the results in two blocks: **failures** (exit 1 — the bundle
   is not v0.1 conformant; each FAIL line with the file and reason) and
   **warnings** (permissive findings — broken links, missing recommended
   fields — that a consumer must tolerate). Suggest concrete fixes for
   failures.

4. Fix nothing automatically. Warnings are NEVER errors — do not "fix"
   them unprompted; broken links may be not-yet-written knowledge.
