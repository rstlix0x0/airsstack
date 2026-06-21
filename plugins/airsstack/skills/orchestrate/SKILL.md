---
name: orchestrate
description: Use when driving a scoped implementation task through the review pipeline — runs coder → reviewer → verifier (with explorer to locate code first) from the main thread and holds the user commit gate. Invoke when a change is substantial enough to warrant test-driven implementation plus an independent review and claim-audit before commit. Soft-coupled to the airsstack agents; if they are not installed, fall back to doing the work inline and tell the user.
---

# Orchestrate

The driver for the `coder → reviewer → verifier` pipeline. Plugin agents are **leaves** — they have no `Agent` tool and cannot chain themselves. So the chaining lives HERE, on the main thread that runs this skill. You are the orchestrator: you spawn each agent, route every finding, and hold the commit gate.

## The four agents

| Agent | Model | Role |
| --- | --- | --- |
| `explorer` | haiku | Read-only locator: finds and maps code as `file:line` tables. Run FIRST when the task needs code located. Refuses judgment. |
| `coder` | sonnet | Implements one scoped task with strict TDD, runs the active stack's DoD, leaves changes in the working tree. Never commits. |
| `reviewer` | opus | One combined report: re-runs the DoD + reviews the diff for style/correctness, AND reviews against the spec/plan intent. Report-only. |
| `verifier` | opus | Audits the coder + reviewer claims against ground truth; emits a VERIFIED/REFUTED/UNCONFIRMED ledger. Report-only, runs once at the gate. |

Namespaced as `airsstack:coder`, `airsstack:reviewer`, etc. Spawn each via the `Agent` tool's `subagent_type`, pinning `model:` per the table above — never let an agent inherit a cheaper default.

## The flow

1. **Locate (optional).** If the task needs code found or a directory mapped first, spawn `explorer`. Use its `file:line` tables to scope the coder's brief. Skip when the target is already clear.
2. **Implement.** Spawn `coder` with one scoped task. Independent tasks → parallel coder spawns (one per task, no shared state).
3. **Review.** On the coder's change receipt, spawn `reviewer` over the diff. It returns the DoD result + code findings + the spec/plan compliance verdict in one report.
4. **Fix loop.** Route every reviewer finding back through YOU to a FRESH `coder` spawn. The reviewer never calls the coder; you do. Repeat steps 3–4 until the review is clean.
5. **Verify.** When the review is clean, spawn `verifier` ONCE over the accumulated coder + reviewer receipts. A REFUTED claim routes back through you to a fresh coder (return to step 3). The verifier never fixes.
6. **Commit gate.** Show the USER the diff + reviewer findings + the verifier ledger and wait for explicit approval. No agent commits — you don't either until the user says so.

## Context handoff

Subagents report through the filesystem so the main thread holds summaries, not full detail. Drive it:

1. **Session start.** Run `scripts/handoff.sh init` once at the top of the pipeline. It prints the
   session dir and id; keep them. It also prunes stale prior sessions and writes the `.active` lease.
2. **Per spawn.** Assign the spawn a file `<NN>-<agent>-<slug>.md` under the session dir and pass that
   **full write-path** in the agent's brief. Call `scripts/handoff.sh beat <session-dir>` as a heartbeat
   so a long run is never pruned by a concurrent session.
3. **On return.** The agent returns its `<summary>` + the relative handoff path — NOT the detail. Route
   off the summary. Pull `<detail>` (read the file yourself) only when YOU must judge it.
4. **Downstream needs detail.** Pass the upstream `handoff:` path plus a targeted `need:` pointer in the
   next agent's brief; it reads the slice into its own context. Detail never transits you unless you must
   reason over it.
5. **Session end.** `scripts/handoff.sh end <session-dir>` drops the lease (optional; the grace window
   self-heals a crash).

The full protocol — file schema, contract, retention — is `process-guidelines/references/context-handoff.md`.

## Invariants (keep these — they are the point of the flow)

- **Flat / leaf.** No agent spawns another agent. Every result passes through you, so the user gate is never bypassed. If you find yourself wanting an agent to "just call the coder," that's the violation — you make the call.
- **Findings route through the orchestrator.** A reviewer or verifier reports; you decide and re-spawn. Reviewers never edit; the verifier never fixes.
- **Validate before trust.** A coder's "DoD green" or a reviewer's "approved" is a claim, not proof — that is exactly why the verifier exists. Cross-check the receipts against each other and against ground truth before the gate.
- **User is the commit gate.** No agent runs `git commit`. You present; the user approves.

## Selective delegation

Delegation trades total token spend for main-context longevity — each spawn re-pays a fixed overhead. So:

- Delegate genuinely heavy work: multi-file implementation, full-diff review, broad locating.
- Keep trivia inline on the main thread (a one-line edit costs more delegated than done directly).
- Don't blanket-delegate to "keep context clean" — it raises total cost for no gain.

## Soft coupling / fallback

This skill assumes the `airsstack` agents are installed. If they are not (the agents don't resolve), degrade gracefully: do the implementation + verification inline on the main thread, follow the same discipline (TDD, DoD from the guidelines skill, user commit gate), and tell the user the agent pipeline was unavailable. Never fail hard for want of the agents.

## Anti-patterns

- An agent spawning another agent (recursion hides steps, bypasses the gate). Agents have no `Agent` tool — keep it that way.
- A coder diff going straight to commit without a reviewer pass.
- A reviewer that edits files instead of reporting; a verifier that proposes fixes instead of auditing.
- Any agent running `git commit`.
- Reaching the commit gate without the verifier pass — the user then has only the agents' word the work is real.
- Blanket-delegating trivia to "keep main context clean" — raises total token spend for no real gain.
