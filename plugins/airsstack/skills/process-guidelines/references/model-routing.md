# Model Routing for Delegated Agents

Governs **delegated agents only** — anything spawned via the `Agent` tool. It cannot change the main
loop, which runs on the session model the user picked. If a trivial task is faster inline, do it
inline; this applies once you have chosen to delegate.

Two independent dials. `model:` is capability, `effort:` is thinking budget — set both.

| Tier | `model` | Use for |
|------|---------|---------|
| Opus | `opus` | Reviewing, analyzing, debugging, design, spec synthesis — anything where a wrong conclusion is expensive. |
| Sonnet | `sonnet` | Execution: writing code, editing files, running tests, applying a known plan. |
| Haiku | `haiku` | Trivia only, per the boundary below. |

`effort:` takes `low` / `medium` / `high` / `xhigh` / `max`, or an integer, and is the dial to move
first when trading cost against depth. Drop it only where the task is narrow and mechanical; keep it
up wherever a wrong answer is expensive. The three bundled agents set theirs in frontmatter:
`explorer` low, `coder` high, `reviewer` high.

Effort is definition-only — the `Agent` tool's spawn parameters expose `model` but not `effort`, so a
per-spawn override is not available.

## Haiku boundary (narrow)

Haiku is permitted ONLY where nothing turns on code logic, design, or review judgment: locating code
and returning `file:line` tables, drafting a commit message from a staged diff, mechanical file
operations, summarizing a grep, whitespace cleanup.

The moment a task reads code to decide *what* to do — locate-then-evaluate is the common case — it has
left the boundary. Escalate to Sonnet (executing) or Opus (judging). Match the dominant verb when
unsure, and prefer Opus on a mixed task.

Never downgrade review, debug, analyze, or design below Opus to save tokens. Recover tokens by scoping
that work tightly, or by lowering `effort` — not by dropping the tier.
