# airsstack

The execution engine for a spec-driven, review-gated development methodology, packaged as a Claude
Code plugin. It ships the agents and the orchestration skill that turn a plan into reviewed,
verified changes — plus process guidelines, project memory, and a verbosity mode.

Language-agnostic: the agents obtain their Definition-of-Done and rules from whichever
`*-guidelines` skill you have installed (e.g. `airsstack-guideline-rust`), and degrade gracefully
when none is present.

## Install

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack@airsstack
```

All components are namespaced `airsstack:<name>`.

## Agents

Spawned by the `orchestrate` skill (or directly via the Agent tool). Each pins its model tier.

| Agent | Model | Role |
| --- | --- | --- |
| `coder` | sonnet | Executes one scoped task with strict TDD, runs the active guideline DoD, never commits. |
| `reviewer` | opus | Re-runs the DoD and reviews the diff for style + correctness + spec/plan intent. Report-only. |
| `verifier` | opus | Audits the phase's accumulated claims against ground truth; emits a VERIFIED/REFUTED/UNCONFIRMED ledger. Report-only. |
| `explorer` | haiku | Read-only locator: returns `file:line` for "where is X". Refuses judgment. |

Agents are leaves — they never spawn other agents. Chaining lives in `orchestrate`.

## Skills

| Skill | Purpose |
| --- | --- |
| `orchestrate` | Drives `explorer → coder → reviewer → verifier → user` per task; routes findings through the orchestrator; the user is the only commit gate. |
| `process-guidelines` | Conventional Commits (workspace-aware scope), model-routing, and the agent-orchestration flow. |
| `concise` | Verbosity-reduction mode (lite / full / ultra). Clean professional terseness, not caveman-speak; persists across the session. |
| `snapshot-load` | Reads the project-local snapshot(s) relevant to the current branch and reports the rehydrated state. |
| `snapshot-save` | Captures a conversation snapshot (session summary + key snippets) into the project-local snapshot store, with a durability gate so thin sessions write nothing. |

## Output style

`terse` — the native, on-demand path to denser output. Toggle with `/output-style`. (For a
persistent, level-based version, use the `concise` skill instead.)

## Hooks

- `SessionStart` (startup / resume / clear) → nudge to run `/airsstack:snapshot-load`.
- `SessionEnd` → nudge to run `/airsstack:snapshot-save`.
- `UserPromptSubmit` → re-inject the active `concise` level each turn (persistent terse mode; no-op
  when no level is active).

The session hooks **nudge only** — you (the model) keep the selection and durability judgment.

### Concise hook runtime

The `UserPromptSubmit` hook prefers `python3` and falls back to `node` (which Claude Code always
ships), exiting silently if neither is found. It is therefore effectively zero-extra-dependency —
install `python3` only if you want the preferred path; nothing breaks without it.

## Project snapshots

`snapshot-save` writes timestamped conversation snapshots (session summary + key snippets) to a
store **outside the repo**, at `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/` (same
user-global root the `concise` hook uses), with a custom `index.md`. `<project-key>` is derived from
`git rev-parse --git-common-dir`, so **all worktrees of one repo share a single store** and snapshots
survive worktree teardown, branch churn, `target/` cleans, and `/clear`. Because it lives outside the
repo, it can never be accidentally committed.

This store is **deliberately separate from Claude's native memory tool** (`~/.claude/projects/.../`
+ `MEMORY.md`), whose store has size limits we are working around — these skills never write there,
and the index is named `index.md`, never `MEMORY.md`.

This is deliberately **local persistence, not git-shareable** — snapshots do not travel to
teammates, CI, or a fresh clone. If you need shared project knowledge, commit it as source (docs,
ADRs), not as a snapshot.

## License

Apache-2.0. See [LICENSE](./LICENSE).
