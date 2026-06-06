# Artifact Lifecycle

How specs, plans, and RFCs are organized, how they relate, and when a plan is safe to
delete. Under the SDD layout the entire artifact tree is **git-ignored**, so plan deletion
is irreversible — read this before removing any plan.

## Why this matters

Once multiple objectives are in flight, a flat artifact directory stops being scannable:
you cannot tell which plan belongs to which spec, which work is in-progress versus
complete, or which plans are safe to clean up. And without a deletion policy, plans
accumulate as dead scaffolding long after the work they described has shipped. This
document fixes both problems: it defines what granularity artifacts should have and when
plans may be removed.

## Where artifacts live

All SDD artifacts live under the per-project, git-ignored tree defined in
`../../../references/artifact-paths.md` (the prose single source of truth for these
paths). In summary:

- RFCs → the `rfcs/` directory (human-authored input, read-only to the plugin)
- Specs → the `specs/` directory (`YYYY-MM-DD-<topic>.md`)
- Plans → the `plans/` directory (`YYYY-MM-DD-<topic>.md`)
- Archived plans → the `plans/_archive/` directory

Any sub-directory layout beyond this — organising artifacts by component, package, or
domain — is a project-local choice and is not imposed here. Keep the
`YYYY-MM-DD-<topic>` naming so it stays the scannable identifier.

## Specs are durable, plans are derived

A **spec** captures the intent and design for a feature or objective. It is the
long-lived record: when decisions made during implementation diverge from the original
spec, those amendments are folded back so the spec always reflects what was actually
built. Specs are not auto-deleted.

Because the artifact tree is git-ignored, a spec's durability is **local to the working
tree** — it is not committed to git history. Treat the spec file as the working record of
intent, and push any decision that must outlive the working tree into a committed durable
location (see Gate 2).

A **plan** is derived from a spec. It is execution scaffolding: a task-by-task
construction manual that serves the implementer during the work. Once the work ships, the
plan's primary value is gone. Plans are deletion candidates once their associated spec is
the source of truth — but deletion must pass three gates (below).

## RFCs are human-owned input

An **RFC** is authored by a human, outside the plugin, and dropped into `rfcs/` as design
input for `brainstorm`. The plugin reads RFCs and never writes, moves, or deletes them.
RFCs are git-ignored like everything under the tree; sharing an RFC across machines is the
engineer's responsibility, out of band. Spec and plan cleanup never touches `rfcs/`.

## One objective per plan

A plan file covers **exactly one objective**: one coherent outcome that can be stated in a
single sentence without an "and". If the goal sentence needs "and" to connect two distinct
outcomes, it describes more than one objective — split it into separate plan files.

A spec that covers multiple objectives produces multiple plan files, one per objective.
Each plan file is independently completable, reviewable, and deletable. Tasks within a
plan are not objectives — several tasks may each implement part of the same feature and
still belong in one plan, because they all serve one goal.

Sibling plan files from the same spec should be named to distinguish their topics:
`2026-06-01-auth-token-validation.md` and `2026-06-01-auth-session-management.md` are
clear; `2026-06-01-auth-plan-1.md` and `2026-06-01-auth-plan-2.md` are not.

## Deletion lifecycle — three gates

A completed spec's plans **may** be deleted once all three of the following gates pass.
All three are required; passing two out of three is not enough.

### Gate 1 — the spec is the source of truth

Every amendment discovered during implementation has been folded back into the spec. The
spec must read as if it always described exactly what shipped — no stale sections, no
undocumented decision divergences. If the spec still has outstanding amendments, fold them
first.

### Gate 2 — durable decisions are in committed source control

Any decision in the plan that belongs permanently in the project has been copied to a
committed durable location: project documentation, configuration, a rules file, or
project memory. This gate matters more under the git-ignored layout: the spec itself is
**not** a committed durable location, so "the spec explains why we chose X" does not
satisfy this gate. A plan that holds the only record of a key architectural decision
cannot be deleted until that decision is captured somewhere committed.

### Gate 3 — manual, per-spec judgment

Deletion is a deliberate decision made for one completed spec's plans, not an automatic
sweep. Check each plan individually. A spec that appears complete can reopen — a bug
report, a follow-on feature, or a compliance requirement may make the original plan
valuable again. If you are unsure whether the recall value is low enough to justify
deletion, archive instead.

## Irreversibility — archive is the default

The artifact tree is git-ignored, so a deleted plan **cannot** be recovered from git
history. Deletion is permanent. Therefore:

- When recall value is even slightly unclear, **archive instead of deleting**: move the
  file to the `plans/_archive/` directory. It stays local-only, costs nothing, and keeps
  the construction trail intact.
- Before deleting outright, get explicit confirmation from whoever owns the work.
- Never delete plans in an automated sweep. The decision is made once per spec, by a
  person, after all three gates pass.

## Anti-patterns

- A plan file whose goal sentence contains "and" joining two distinct outcomes — it
  carries two objectives and cannot be cleanly deleted or reviewed as one unit. Split it.
- Deleting a plan whose spec still has unmerged amendments or undocumented decisions — the
  plan is the only record of those decisions until they are folded back into a committed
  location.
- An automatic "delete all completed plans" sweep — deletion is per-spec, confirmed,
  deliberate.
- Deleting rather than archiving when recall value is unclear — git history cannot recover
  an ignored file.
- Skipping the three-gate check because the work "obviously shipped" — the gates exist
  precisely for cases that feel obvious.
- Editing, moving, or deleting a file under `rfcs/` — RFCs are human-owned input the
  plugin only reads.
