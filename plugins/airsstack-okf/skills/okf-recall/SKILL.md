---
name: okf-recall
description: >
  Answer a question from the repo's OKF knowledge bundle with progressive
  disclosure: read index.md first, open only the needed concepts for
  targeted lookups, or spawn the read-only okf-recall agent for broad
  queries and receive compact pointers. Use when the user says "what does
  the knowledge bundle say about X" / "/okf-recall X".
---

# okf-recall

Consume the bundle cheaply. index.md is the disclosure layer — never
start by globbing concept bodies.

## Steps

1. Resolve the bundle root:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/okf-root.sh" [explicit-path]
   ```

   Exit 2 → relay stderr and STOP.

2. Read `<root>/index.md` ONLY. If it is missing, synthesize a view with
   a Glob over `<root>/**/*.md` (spec-sanctioned) and continue.

3. Judge breadth from the index descriptions:

   - **Targeted** — the answer plausibly lives in ≤3 concepts: open
     exactly those, answer, cite their concept IDs.
   - **Broad** — many candidates or graph traversal needed: spawn the
     `okf-recall` subagent (Agent tool, `subagent_type: okf-recall`)
     with `query`, `bundle_root`, and `spec_digest` =
     `${CLAUDE_PLUGIN_ROOT}/references/okf-spec.md`. Relay its pointers
     and summary; open at most ONE more concept if you still need it.

4. Tolerate spec-tolerated degradation silently: unknown types, missing
   fields, broken links (mention a broken link only if it blocked the
   answer).

## If the agent is unavailable

Degrade to targeted mode with a wider read budget, tell the user, and
never fabricate bundle content.
