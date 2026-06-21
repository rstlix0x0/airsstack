---
name: process-guidelines
description: Use when committing or delegating work to subagents — supplies the project's process conventions, namely Conventional Commits with workspace-aware scopes, model-routing for delegated agents (judge vs execute vs trivia), and the agent-orchestration flow. Read the matching reference before applying a convention.
---

# Process Guidelines

The conventions for *how the work gets done* — commits and delegation. `SKILL.md` carries the
load-bearing one-liners; the detail lives in `references/` and is read on demand.

> Spec and plan artifact conventions (one objective per plan, durable specs vs disposable plans,
> the deletion lifecycle) live with the workflow that produces them — see the `airsstack-sdd` plugin.

## The essentials

- **Commits** follow Conventional Commits with a workspace-aware scope: `type(scope): summary`.
  Scope is your own package/member name (or `workspace` / `repo` for cross-cutting changes). Imperative
  mood, subject ≤72 chars, body explains *why*. Every commit passes the stack's Definition of Done on
  its own. → `references/conventional-commits.md`
- **Model routing** for delegated agents: **Opus** judges (review, debug, analyze, design, verify),
  **Sonnet** executes (write, edit, run, apply a known plan), **Haiku** for narrow non-judgment trivia
  only (locate, summarize a grep, draft a commit message from a staged diff). Pin the tier explicitly on
  every delegated coding/review/thinking agent; never downgrade judgment to save tokens.
  → `references/model-routing.md`
- **Agent orchestration**: agents are **leaves** (no agent spawns another); the chain runs FLAT on the
  main thread; findings route back through the orchestrator to a fresh coder; the user is the sole commit
  gate. The operational driver is the `orchestrate` skill. → `references/agent-orchestration.md`
- **Context handoff**: subagents report through the filesystem — a cheap `<summary>` returns to the main
  thread, heavy `<detail>` stays on disk and is pulled by path only when needed. Sessions are managed by
  `scripts/handoff.sh` (`init`/`beat`/`end`). → `references/context-handoff.md`

## Reference index

Read the one that matches your task:

- `references/conventional-commits.md` — full type table, scope selection, breaking-change signalling,
  body rules, anti-patterns.
- `references/model-routing.md` — the tier table, the narrow Haiku boundary, how to apply the tier on a
  spawn, and what not to downgrade.
- `references/agent-orchestration.md` — the leaf invariant, the flow, selective delegation,
  validate-before-trust, and where the commit gate sits.
- `references/context-handoff.md` — the handoff path layout, file schema, the summary+path return and
  path-pointer routing contract, and the `handoff.sh` session lifecycle.
