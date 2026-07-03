---
name: okf-enrich
description: >
  Batch-produce OKF concept documents for a source scope (a crate, module,
  or docs directory) by spawning the isolated okf-enricher agent, then
  regenerate index.md and lint the bundle. Use when the user says "enrich
  the knowledge bundle from X" / "/okf-enrich X".
---

# okf-enrich

Heavy production runs in the isolated enricher; the main thread
orchestrates, regenerates the index, and closes the loop with lint.

## Steps

1. Resolve the bundle root:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/okf-root.sh" [explicit-path]
   ```

   Exit 2 → relay stderr and STOP.

2. Pin down the scope with the user if ambiguous (which directory, any
   focus notes). Do not read the scope's files yourself — that is the
   agent's job.

3. Spawn the `okf-enricher` subagent (Agent tool,
   `subagent_type: okf-enricher`), passing: `scope`, `bundle_root`
   (absolute), and `spec_digest` =
   `${CLAUDE_PLUGIN_ROOT}/references/okf-spec.md`.

4. On receipt, run both scripts:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/gen-index.sh" "<root>"
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/okf-lint.sh" "<root>"
   ```

   Lint-after-enrich is the producer feedback loop: hard failures mean
   the enricher emitted nonconformant documents — fix them (or re-spawn
   with the failure list) before reporting success.

5. Surface to the user: the agent's receipt (concept IDs, log entries,
   gaps) plus the lint summary. Do NOT commit.

## If the agent is unavailable

Fall back to doing pass 1 / pass 2 inline in the main thread (same rules,
same log discipline), and tell the user the isolation was skipped.
