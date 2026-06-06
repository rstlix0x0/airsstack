# Model Routing for Delegated Agents

How to pick the model tier when delegating work to subagents. This governs **delegated agents only** —
anything spawned via the `Agent` tool or a workflow `agent()` call. It does NOT control the main
conversation loop, which runs on the session model the user selected; a routing rule cannot downgrade
that mid-session. "Use Haiku for commit messages" means *spawn a Haiku subagent for it*, not *the main
loop becomes Haiku*. If a trivial task is faster done inline, do that — the table applies when you choose
to delegate.

This exists because cheaper tiers silently win by default: an agent that inherits an unannotated default
may run below the tier its task needs. The fix is to make the tier an explicit, reviewed decision.

## The mapping

| Tier | `model` alias | Use for |
|------|---------------|---------|
| Opus | `opus` | Thinking, analyzing, reviewing, debugging, architecture/design, adversarial verification, spec/plan synthesis. Anything where a wrong conclusion is expensive. |
| Sonnet | `sonnet` | Execution: writing code, editing files, running tests, applying a known plan, mechanical refactors with a clear target. |
| Haiku | `haiku` | Out-of-scope-for-engineering trivia only — see the boundary below. |

### Haiku boundary (narrow)

Haiku is permitted ONLY for tasks that touch no code logic, no design, and no review judgment:

- Locating code / mapping a directory and returning `file:line` tables (no evaluation).
- Drafting a commit message from an already-staged diff.
- Mechanical file operations (move/rename/delete a named file, stage paths).
- Simple lookups and summarizing a grep/`ls` result.
- Formatting / whitespace / text cleanup with no semantic decision.

Anything that reads code to form a judgment, locates-and-then-evaluates, writes or reviews logic, or
reasons about design is NOT Haiku — it is Sonnet (executing) or Opus (judging). When a "simple" task
turns out to require reading code to decide *what* to do, it has left the Haiku boundary; escalate.

### When unsure

Match the **dominant verb**. "Review / analyze / debug / think / verify" → Opus. "Write / edit / run /
apply" → Sonnet. Never route review, debug, analyze, or design below Opus to save tokens — that is the
one downgrade this rule forbids. If torn between Sonnet and Opus for a mixed task, pick Opus.

## Canonical tier assignment (the four agents)

The bundled agents fix their tier by role: `coder` = sonnet (executes), `reviewer` = opus (judges),
`verifier` = opus (audits), `explorer` = haiku (locates only, refuses judgment — the constraint that
keeps it inside the Haiku boundary).

## How to apply

The tier MUST be explicit on every delegated agent that does coding, review, thinking, analysis, or
debugging. Do not rely on an `agentType` or workflow default for those.

- **`Agent` tool:** pass `model: 'sonnet' | 'opus' | 'haiku'` on the spawn.
- **Workflow per-agent:** `agent(prompt, { model: 'opus', ... })`; the `model` override takes precedence
  over the agent definition's frontmatter. Mirror it on the matching phase entry so the tier is visible.
- **Custom agent definitions:** pin `model:` in frontmatter when the agent has a fixed role.
- Trivia agents MAY set `haiku` explicitly — prefer explicit so intent is auditable.

## Interaction with token suppression

Spend deliberately on the high-stakes tiers (Opus for judgment, Sonnet for execution) and recover tokens
on genuine trivia (Haiku). A wrong review or analysis costs far more than the Opus tokens that would have
caught it. Suppress tokens by *scoping* the Opus/Sonnet work tightly, not by downgrading work that needs
the strong tier.

## Anti-patterns

- A review / debug / analysis / design agent with no `model:`, inheriting a cheap default. Pin `opus`.
- Routing a "locate where X is and decide if it's a bug" task to Haiku because it "looks like a search".
  Locating-to-judge is Opus/Sonnet, not Haiku.
- Downgrading an Opus-tier task to save tokens. Forbidden for review/debug/analyze/design.
- Using a full model ID string where the `model` param expects the `opus`/`sonnet`/`haiku` alias.
