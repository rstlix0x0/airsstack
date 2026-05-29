# Agent Orchestration Flow

How delegated agents chain together and where the human gate sits. Loads unconditionally — it governs every multi-agent flow in this repo. Sibling to `ai-model-routing.md`: that rule picks the model *tier*; this rule picks the *flow*. (`ai-*` prefix = rules about how AI/agents operate.)

## Why this rule exists

Delegation without a fixed flow drifts: a coder's diff reaches a commit unreviewed, a reviewer starts editing files instead of reporting, or an agent spawns another agent and the human approval gate disappears. This rule fixes the flow so every delegated change passes through review and through the user before it lands.

## The agents

Four repo-owned agents under `.claude/agents/airsstack-*`:

| Agent | Model | Role |
| --- | --- | --- |
| `airsstack-coder` | sonnet | Executes one scoped task: writes code + tests (strict TDD), runs the DoD, leaves changes in the working tree. Never commits. |
| `airsstack-code-reviewer` | opus | Re-runs the DoD, reviews the diff against `.claude/rules/` + for correctness. Report-only. |
| `airsstack-spec-reviewer` | opus | Reviews the implementation against spec/plan intent. Report-only. |
| `airsstack-verifier` | opus | Audits the phase's accumulated claims against ground truth (cargo/git/Read); emits a VERIFIED/REFUTED/UNCONFIRMED ledger. Report-only, runs once at the final gate. |

Model tiers are fixed by `ai-model-routing.md`. This rule fixes how the agents connect.

## Agents are leaves

- A spawned agent does NOT spawn another agent. None of the four carry the `Agent` tool.
- All chaining lives in the orchestrator: the main thread, or a one-level `Workflow` script.
- A reviewer never calls a coder. The orchestrator takes the reviewer's findings and spawns a fresh coder. Every result passes through the orchestrator, so the approval gate is never bypassed.

## The flow

1. Phase / task start → orchestrator spawns `airsstack-coder` per scoped task. Independent tasks → parallel coder spawns.
2. Coder returns its change receipt → orchestrator spawns `airsstack-code-reviewer` on the diff.
3. Phase boundary → orchestrator spawns `airsstack-spec-reviewer` for the intent check. code-reviewer and spec-reviewer have independent inputs and MAY run in parallel.
4. Findings route back through the orchestrator to a fresh coder spawn for fixes. Repeat 2–4 until clean.
5. Reviews clean → orchestrator spawns `airsstack-verifier` ONCE on the phase's accumulated receipts (coder + both reviewers). It re-checks each claim against ground truth and returns a VERIFIED/REFUTED/UNCONFIRMED ledger. A REFUTED claim routes back through the orchestrator to a fresh coder (return to step 2); the verifier never fixes.
6. Commit gate → orchestrator shows the USER the diff + reviewer findings + the verifier ledger and waits for explicit approval. No agent commits. (See `git-commits.md` and the dev-rule: no commit without approval.)

## Selective delegation

Delegation trades total token spend for main-context longevity. Every spawn re-pays ~20k fixed overhead (system prompt + tool defs + CLAUDE.md + rules). So:

- Delegate genuinely heavy work: multi-file writes, full-diff reviews, broad searches.
- Keep trivia inline on the main thread, or send it to a Haiku spawn (per `ai-model-routing.md`).
- Do NOT blanket-delegate — a one-line edit costs more total tokens delegated than done inline.

## Validate before trust

Before a new or changed agent gates real work, dry-run it on known-good material (e.g. an already-merged diff) and confirm its output schema and judgment. A misfiring agent caught here never blocks or corrupts real work. Note: a custom agent under `.claude/agents/` becomes spawnable only in a session that started with the file present — author, then validate in a fresh session before relying on it.

## How to apply

- Drive the coder → code-reviewer → spec-reviewer → user-approval pipeline from the main thread or a one-level `Workflow` script. Never from inside an agent.
- Spawn agents by name (`airsstack-coder`, etc.) via the `Agent` tool's `subagent_type`, or as a workflow `agentType`. Pin `model:` explicitly per `ai-model-routing.md`.
- Route every reviewer finding back through the orchestrator. Never wire a reviewer to edit or to spawn.
- Hold the commit until the user approves the reviewed diff.

## Anti-patterns

- An agent spawning another agent (recursion hides steps, bypasses the gate). Forbidden — agents have no `Agent` tool.
- A coder diff going straight to commit without a code-review pass. Forbidden.
- A reviewer that edits files or proposes large refactors instead of reporting findings. Reviewers are report-only.
- Any agent running `git commit`. The user is the commit gate.
- Blanket-delegating trivia to "keep main context clean" — it raises total token spend for no real gain.
- Reaching the commit gate without running `airsstack-verifier` over the phase's claims — the user then has only the agents' word that the work is real. Forbidden for phase-boundary work.
- The verifier marking an unverifiable claim VERIFIED to be agreeable, or proposing fixes instead of reporting. It audits; it does not edit.

## Definition of Done (rule additions)

- Every delegated change passed through `airsstack-code-reviewer` before reaching the user.
- Phase-boundary work passed through `airsstack-spec-reviewer`.
- The phase's claims passed through `airsstack-verifier` before the commit gate; any REFUTED claim was resolved (re-coded + re-reviewed), not waved through.
- No agent committed; the user approved the diff before commit.
- The flow ran flat — no agent-to-agent spawn.

Cross-links: `ai-model-routing.md` (model tier per agent), `git-commits.md` (the user commits, following the convention), `rust-strict-quality.md` (the DoD the code-reviewer re-runs).
