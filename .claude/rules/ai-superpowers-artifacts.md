# Superpowers Artifacts — Layout & Lifecycle

How `.superpowers/` specs and plans are organized and when they get removed. Loads unconditionally — it governs every brainstorm/plan the `superpowers` workflow produces in this repo. This rule extends the "Superpowers artifact paths" override in root `CLAUDE.md`; that note fixes the directory, this rule fixes the per-tier sub-layout and the deletion policy.

## Why this rule exists

`.superpowers/` is a flat scratch dir. With more than one unit of work in flight, a flat `specs/` and `plans/` stop being scannable — you cannot tell at a glance which artifact belongs to which crate or to repo tooling. And plans accumulate forever even after the work they describe has shipped, so the dir fills with dead scaffolding. This rule tiers the layout and defines when a plan is safe to delete.

## Layout — tier the artifacts

Both trees are sub-divided by the **same tier vocabulary as commit scopes** (see [[git-commits]]):

```
.superpowers/
├── specs/
│   ├── clauders/      # spec for the clauders crate
│   ├── workspace/     # spec spanning root config / bootstrap
│   └── repo/          # spec for repo tooling (.claude/ agents, skills, rules)
└── plans/
    ├── clauders/
    ├── workspace/
    └── repo/
```

- **Tier** is the workspace member name (`clauders` today), or `workspace` (root Cargo files / multi-crate bootstrap), or `repo` (`.claude/`, `.github/`, docs). Add a new tier directory only when a new workspace member exists — keep it in sync with the [[git-commits]] scope vocabulary.
- **Filename** is `YYYY-MM-DD-<topic>.md`. The tier lives in the directory, so do NOT repeat it in the filename (`plans/clauders/2026-05-28-phase-1-workspace.md`, not `…-clauders-phase-1-…`).
- **Tiebreak** when an artifact spans tiers: file it under its dominant subject, same rule as a multi-crate commit scope. A "workspace bootstrap for clauders" spec is dominantly about clauders → `specs/clauders/`.

## Granularity — one objective per plan

A plan file describes **exactly one objective** — one coherent outcome the execution drives toward. The plan MAY break that objective into multiple tasks/phases, but all of them serve the single objective.

- **One objective = one plan file.** If you cannot state the plan's goal in a single sentence without "and", it is more than one objective — split it.
- **A spec with multiple objectives produces multiple plan files**, one per objective, all under the spec's tier dir. They share the spec as their common source of intent.
- **Tasks are not objectives.** "Add the newtype, wire the builder, write the round-trip tests" are three tasks under one objective ("ship the validated `Foo` type"). They stay in one plan.
- **Naming** disambiguates sibling plans of one spec by topic, not by number-in-isolation: `plans/clauders/2026-05-28-streaming-transport.md` and `…-streaming-sse-parser.md`, not a single `…-streaming.md` carrying two objectives.

Why: a single-objective plan is independently completable, reviewable, and deletable (see lifecycle below). A multi-objective plan cannot be deleted when only half its work shipped, and it muddies the spec→plan trace.

## Lifecycle — plans are derived, specs are the record

- A **spec** captures intent/design. It is the durable artifact. Specs are NOT auto-deleted.
- A **plan** is derived from a spec — disposable scaffolding for execution, scoped to one objective (see above). Once the spec's work has shipped, its plans are deletion candidates.

### Deleting completed plans

When every plan for a spec has been delivered, the plans MAY be deleted — but only after all three gates pass:

1. **Spec is the source of truth.** Every in-flight amendment discovered during execution has been folded back into the spec (the repeated "spec amended §X" practice). The spec must read as if it always described what shipped.
2. **Durable decisions are in source control.** Anything in the plan that belongs in the repo has been copied to `CLAUDE.md`, a `.claude/rules/` file, real `docs/`, or project memory — per the `CLAUDE.md` "leave the generated artifact alone" rule. Phase-delivery memory snapshots count.
3. **Manual, per-spec judgment.** Deletion is a deliberate decision for one completed spec's plans, never an automatic sweep. A "completed" spec can reopen.

### Deletion is irreversible — `.superpowers/` is gitignored

There is no git history to recover from. A deleted plan is gone permanently. Therefore:

- Get explicit user confirmation before deleting plans. Treat it like any irreversible action.
- If recall value is uncertain, prefer archiving over deleting: move to `.superpowers/plans/_archive/<tier>/`. Costs ~nothing (still gitignored, local-only) and keeps the "how it was built" trail that memory snapshots only summarize.

## How to apply

- New brainstorm → `.superpowers/specs/<tier>/YYYY-MM-DD-<topic>.md`.
- New plan → `.superpowers/plans/<tier>/YYYY-MM-DD-<topic>.md`, scoped to **one objective**. A multi-objective spec gets one plan file per objective.
- The `superpowers` skills still default to a flat path; place the artifact in the correct tier sub-dir yourself.
- Spec shipped → fold amendments back into the spec, confirm durable decisions are in source control, then ask the user before deleting (or archiving) its plans.
- Never `git add` anything under `.superpowers/` — it stays gitignored.

## Anti-patterns

- A plan file carrying two or more objectives (goal sentence needs an "and"). Split into one plan per objective.
- A flat `specs/` / `plans/` once a second unit of work exists. Tier it.
- Repeating the tier in the filename (`clauders/…-clauders-…`). Redundant.
- Deleting a plan whose spec still has unmerged amendments or undocumented decisions. Fold back first.
- An automatic "delete all completed plans" sweep. Deletion is per-spec, confirmed, deliberate.
- Deleting (vs archiving) when recall value is unclear — there is no git undo here.

Cross-links: root `CLAUDE.md` (`.superpowers/` is gitignored scratch; copy durable decisions into source control), [[git-commits]] (same tier vocabulary as commit scopes).
