# Artifact Lifecycle

How specs and plans are organized, how they relate to each other, and when a plan is safe to delete.
Read this before removing any plan — especially if the project's plan directory is not tracked in git,
where deletion is irreversible.

## Why this matters

A flat `docs/specs/` and `docs/plans/` directory works fine for a single unit of work. Once multiple
objectives are in flight, it stops being scannable: you cannot tell at a glance which plan belongs to
which spec, which work is in-progress versus complete, or which plans are safe to clean up. And without
a deletion policy, plans accumulate as dead scaffolding long after the work they described has shipped.
This document fixes both problems: it defines what granularity artifacts should have and when plans may
be removed.

## Specs are durable, plans are derived

A **spec** captures the intent and design for a feature or objective. It is the long-lived record. Once
decisions made during implementation diverge from the original spec, those amendments are folded back
into the spec so it always reflects what was actually built. Specs are not auto-deleted.

A **plan** is derived from a spec. It is execution scaffolding: a task-by-task construction manual that
serves the implementer during the work. Once the work ships, the plan's primary value is gone. Plans are
deletion candidates once their associated spec is the source of truth — but deletion must pass three
gates (see below).

## One objective per plan

A plan file covers **exactly one objective**: one coherent outcome that can be stated in a single
sentence without an "and". If the goal sentence needs "and" to connect two distinct outcomes, it
describes more than one objective — split it into separate plan files.

A spec that covers multiple objectives produces multiple plan files, one per objective. Each plan file
is independently completable, reviewable, and deletable. Tasks within a plan are not objectives —
several tasks may each implement part of the same feature and still belong in one plan, because they
all serve one goal.

Sibling plan files from the same spec should be named to distinguish their topics:
`2026-06-01-auth-token-validation.md` and `2026-06-01-auth-session-management.md` are clear;
`2026-06-01-auth-plan-1.md` and `2026-06-01-auth-plan-2.md` are not.

## Where artifacts live

By default:

- Specs → `docs/specs/YYYY-MM-DD-<topic>.md`
- Plans → `docs/plans/YYYY-MM-DD-<topic>.md`

Any sub-directory layout beyond this — organising artifacts by component, package, or domain — is a
**project-local choice** and is not imposed here. If your project uses sub-directories, keep the naming
conventions consistent so the `YYYY-MM-DD-<topic>` pattern remains the scannable identifier.

## Deletion lifecycle — three gates

A completed spec's plans **may** be deleted once all three of the following gates pass. All three are
required; passing two out of three is not enough.

### Gate 1 — the spec is the source of truth

Every amendment discovered during implementation has been folded back into the spec. The spec must read
as if it always described exactly what shipped — no stale sections, no undocumented decision divergences.
If the spec still has outstanding amendments, fold them first.

### Gate 2 — durable decisions are in source control

Any decision in the plan that belongs permanently in the project has been copied to the appropriate
durable location: project documentation, configuration, a rules file, or project memory. A plan that
contains the only record of a key architectural decision cannot be deleted until that decision is
captured elsewhere. "The plan explains why we chose X" is not sufficient if the plan is about to
disappear.

### Gate 3 — manual, per-spec judgment

Deletion is a deliberate decision made for one completed spec's plans, not an automatic sweep. Check
each plan individually. A spec that appears complete can reopen — a bug report, a follow-on feature, or
a compliance requirement may make the original plan valuable again. If you are unsure whether the recall
value is low enough to justify deletion, archive instead (see below).

## Irreversibility — tracked vs ignored plans

Whether deletion is reversible depends on how the project tracks `docs/plans/`:

**If `docs/plans/` is git-ignored:** deletion is permanent. There is no history to recover from. Before
deleting, get explicit confirmation from whoever owns the work. When recall value is even slightly
unclear, prefer archiving over deleting: move the file to `docs/plans/_archive/` instead. It stays
local-only, costs nothing, and keeps the construction trail intact.

**If `docs/plans/` is tracked in git:** a deleted plan can be recovered from git history. Deletion is
still a deliberate action — confirm before doing it — but the irreversibility risk is lower. Archiving
to `docs/plans/_archive/` is still a reasonable choice when you want the file to remain immediately
accessible without a `git show` lookup.

In either case, do not delete plans in an automated sweep. The decision is made once per spec, by a
person, after all three gates pass.

## Anti-patterns

- A plan file whose goal sentence contains "and" joining two distinct outcomes — it carries two
  objectives and cannot be cleanly deleted or reviewed as one unit. Split it.
- Deleting a plan whose spec still has unmerged amendments or undocumented decisions — the plan is the
  only record of those decisions until they are folded back.
- An automatic "delete all completed plans" sweep — deletion is per-spec, confirmed, deliberate.
- Deleting rather than archiving when recall value is unclear and git history cannot recover the file.
- Skipping the three-gate check because the work "obviously shipped" — the gates exist precisely for
  cases that feel obvious.
