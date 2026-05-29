# Model Routing for Delegated Agents

How to pick the model tier when delegating work to subagents and workflow phases. Loads unconditionally — it governs every `Agent` spawn and `Workflow` script in this repo.

This rule exists because cheaper tiers silently win by default: `agentType`-based agents (e.g. `Explore`) and unannotated workflow phases inherit a default model that may be below the tier the task needs. A review phase once ran on Haiku for exactly this reason. The fix is to make the tier an explicit, reviewed decision.

## Scope — and the one thing this rule can NOT do

This rule controls **delegated agents only**: anything spawned via the `Agent` tool or an `agent()` call inside a `Workflow` script.

It does **not** control the main conversation loop. The main thread runs on the session model the user selected (currently Opus 4.8); a rule cannot downgrade it mid-session. So "use Haiku for commit messages" means *spawn a Haiku subagent for that*, not *the main loop becomes Haiku*. If a trivial task is faster to just do inline on the main thread, do that — the routing table applies when you choose to delegate.

There is no hook that rewrites a model choice (unlike the bash-rewriting hooks). Enforcement is the orchestrator's discipline, expressed through the levers in [How to apply](#how-to-apply). Treat the table as binding.

## The mapping

| Tier   | `model` alias | Use for                                                                                  |
| ------ | ------------- | ---------------------------------------------------------------------------------------- |
| Opus   | `opus`        | Thinking, analyzing, reviewing, debugging, architecture/design, adversarial verification, spec/plan synthesis. Anything where a wrong conclusion is expensive. |
| Sonnet | `sonnet`      | Execution: writing code, editing files, running tests, applying a known plan, mechanical refactors with a clear target. |
| Haiku  | `haiku`       | Out-of-scope-for-engineering trivia only — see the narrow boundary below.                |

### Haiku boundary (narrow)

Haiku is permitted ONLY for tasks that touch no code logic, no design, and no review judgment:

- Drafting a commit message from an already-staged diff.
- Mechanical file operations (move/rename/delete a named file, stage paths).
- Simple lookups and summarizing a grep/`ls` result.
- Formatting / whitespace / lint-text cleanup with no semantic decision.

Anything that reads code to form a judgment, locates-and-evaluates, writes or reviews logic, or reasons about design is NOT Haiku — it is Sonnet (if executing) or Opus (if judging). When a "simple" task turns out to require reading code to decide *what* to do, it has left the Haiku boundary; escalate.

### When unsure

Match the **dominant verb** of the task. "Review / analyze / debug / think / verify" → Opus. "Write / edit / run / apply" → Sonnet. Never route review, debug, analyze, or design below Opus to save tokens — that is the one downgrade this rule forbids. If torn between Sonnet and Opus for a mixed task, pick Opus.

## How to apply

The tier MUST be explicit on every delegated agent that does coding, review, thinking, analysis, or debugging. Do not rely on `agentType` or workflow defaults for those — that is the exact gap that produced a Haiku review.

- **Workflow per-agent:** `agent(prompt, { model: 'opus', ... })`. Set it even when also passing `agentType` — the `model` override takes precedence over the agent definition's frontmatter.
- **Workflow phase display:** add `model` to the matching `meta.phases[]` entry so the phase's tier is visible in progress output.
- **`Agent` tool:** pass `model: 'sonnet' | 'opus' | 'haiku'` on the spawn.
- **Custom agent definitions:** pin `model:` in frontmatter when the agent has a fixed role (a reviewer agent pins `opus`).
- Trivia agents MAY omit `model` (defaults are already cheap) or set `haiku` explicitly; prefer explicit `haiku` so intent is auditable.

Model aliases resolve to: `opus` → Opus 4.8 (`claude-opus-4-8`), `sonnet` → Sonnet 4.6 (`claude-sonnet-4-6`), `haiku` → Haiku 4.5 (`claude-haiku-4-5-20251001`). Use the aliases, not the full IDs, in `model` params.

## Interaction with the token-suppression goal

The root `CLAUDE.md` makes token suppression load-bearing. This rule deliberately spends on the high-stakes tiers (Opus for judgment, Sonnet for execution) and recovers tokens on trivia (Haiku). It is a quality-first stance, not a cost-first one: a wrong review or analysis costs far more than the Opus tokens that would have caught it. Suppress tokens by *scoping* the Opus/Sonnet work tightly and pushing genuine trivia to Haiku — not by downgrading the work that needs the strong tier.

## Anti-patterns

- A review / debug / analysis / design agent with no `model:` set, inheriting a cheap default. Rejected — pin `opus`.
- Routing a "locate where X is defined and decide if it's a bug" task to Haiku because it "looks like a search". Locating-to-judge is Opus/Sonnet, not Haiku.
- Downgrading an Opus-tier task to Sonnet/Haiku to save tokens. Forbidden for review/debug/analyze/design.
- Using the full model ID string where the `model` param expects the `opus`/`sonnet`/`haiku` alias.

## Definition of Done (rule additions)

- Every delegated agent doing coding, review, thinking, analysis, or debugging carries an explicit `model:`.
- No review/debug/analyze/design phase runs below Opus.
- Workflow `meta.phases[]` entries that use a non-default tier declare `model`.
- Haiku appears only on tasks inside the narrow boundary above.

Cross-links: root `CLAUDE.md` (token-suppression goal), `git-commits.md` (commit-message drafting is a Haiku-eligible task), `ai-agent-orchestration.md` (how these routed agents chain + gate). Mirrors the `feedback-subagent-model-routing` memory, which this rule supersedes (Haiku is now allowed for narrow trivia, previously "never Haiku").
