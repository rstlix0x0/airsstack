# Agent Orchestration

How delegated agents chain together and where the human gate sits. This reference states the
**principles**; the operational driver that runs the flow is the `orchestrate` skill — invoke that to
execute, read this to understand the rules it enforces.

## Why this matters

Delegation without a fixed flow drifts: a coder's diff reaches a commit unreviewed, a reviewer starts
editing instead of reporting, or an agent spawns another agent and the human gate disappears. These rules
fix the flow so every delegated change passes through review and through the user before it lands.

## The agents

| Agent | Model | Role |
|-------|-------|------|
| `explorer` | haiku | Read-only locator; finds and maps code. Refuses judgment. |
| `coder` | sonnet | Executes one scoped task with strict TDD, runs the DoD, leaves changes in the tree. Never commits. |
| `reviewer` | opus | Re-runs the DoD, reviews the diff for correctness + style, AND reviews against the spec/plan intent. Report-only. |
| `verifier` | opus | Audits the accumulated claims against ground truth; emits a VERIFIED/REFUTED/UNCONFIRMED ledger. Report-only, once at the gate. |

## Agents are leaves

- A spawned agent does NOT spawn another agent — none of them carry the `Agent` tool.
- All chaining lives in the orchestrator: the main thread, or a one-level workflow script.
- A reviewer never calls a coder. The orchestrator takes the reviewer's findings and spawns a *fresh*
  coder. Every result passes through the orchestrator, so the gate is never bypassed.

## The flow

1. **Locate (optional).** Spawn `explorer` when the task needs code found or a directory mapped first.
2. **Implement.** Spawn `coder` per scoped task. Independent tasks → parallel coder spawns.
3. **Review.** On the coder's receipt, spawn `reviewer` over the diff — one report covering the DoD,
   code correctness/style, and spec/plan compliance.
4. **Fix loop.** Findings route back through the orchestrator to a fresh coder. Repeat 3–4 until clean.
5. **Verify.** Reviews clean → spawn `verifier` ONCE over the accumulated coder + reviewer receipts. A
   REFUTED claim routes back through the orchestrator to a fresh coder (return to step 3). The verifier
   never fixes.
6. **Commit gate.** The orchestrator shows the USER the diff + reviewer findings + the verifier ledger
   and waits for explicit approval. No agent commits.

## Selective delegation

Delegation trades total token spend for main-context longevity — each spawn re-pays a fixed overhead.

- Delegate genuinely heavy work: multi-file writes, full-diff reviews, broad locating.
- Keep trivia inline on the main thread (a one-line edit costs more delegated than done directly).
- Don't blanket-delegate to "keep context clean" — it raises total cost for no gain.

## Validate before trust

Before a new or changed agent gates real work, dry-run it on known-good material (e.g. an
already-reviewed diff) and confirm its output and judgment. A custom agent becomes spawnable only in a
session that started with its file present — author it, then validate in a fresh session before relying
on it.

## Anti-patterns

- An agent spawning another agent (recursion hides steps, bypasses the gate). Forbidden — agents have no
  `Agent` tool.
- A coder diff going straight to commit without a review pass.
- A reviewer that edits files or proposes large refactors instead of reporting.
- The verifier proposing fixes, or marking an unverifiable claim VERIFIED to be agreeable. It audits; it
  does not edit.
- Any agent running `git commit`. The user is the commit gate.
- Reaching the commit gate without running the verifier — the user then has only the agents' word that
  the work is real.
