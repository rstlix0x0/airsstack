# Artifact Lifecycle

How specs, plans, and RFCs relate, and when a plan is safe to delete. No SDD artifact
lives in git history: RFCs sit under the git-ignored `.airsstack/` tree, specs and plans in
the HOME-global store outside any repo. Plan deletion is therefore irreversible — read
§ Irreversibility before removing any plan.

## Where artifacts live

`../../../references/artifact-paths.md` is the single source of truth for the two roots,
the per-repo `<key>`, and every artifact's directory and naming. Read the paths there, not
here.

Any sub-directory layout beyond that scheme — organising artifacts by component, package,
or domain — is a project-local choice and is not imposed here. Keep the
`YYYY-MM-DD-<topic>` naming so it stays the scannable identifier.

## Specs are durable, plans are derived

A **spec** captures the intent and design for a feature or objective. It is the
long-lived record: when decisions made during implementation diverge from the original
spec, those amendments are folded back so the spec always reflects what was actually
built. Specs are not auto-deleted.

Because the HOME-global root is outside any repo, a spec's durability is **per-user
local persistence** — shared across every worktree of the repo and surviving worktree
teardown, but not committed to git history. Treat the spec file as the working record of
intent, and push any decision that must outlive the local store into a committed durable
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

The rule and the sibling-naming convention are stated in `write-plan`'s SKILL.md § Scope
check. What matters here: because each plan covers one objective, each is independently
completable, reviewable, and **deletable** — the gates below apply per plan file.

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
committed durable location: documentation, configuration, a rules file, or project memory.
The store lives outside any repo, so the spec is **not** a committed durable location —
"the spec explains why we chose X" does not satisfy this gate. A plan holding the only
record of a key architectural decision cannot be deleted until that record is committed.

### Gate 3 — manual, per-spec judgment

Deletion is a deliberate decision made for one completed spec's plans, not an automatic
sweep. Check each plan individually. A spec that appears complete can reopen — a bug
report, a follow-on feature, or a compliance requirement may make the original plan
valuable again. If you are unsure whether the recall value is low enough to justify
deletion, archive instead.

## Irreversibility — archive is the default

The HOME-global store is outside any repo and not committed, so a deleted plan **cannot**
be recovered from git history. Deletion is permanent. Therefore:

- When recall value is even slightly unclear, **archive instead of deleting**: move the
  file to the `plans/_archive/` directory. It stays local-only, costs nothing, and keeps
  the construction trail intact.
- Before deleting outright, get explicit confirmation from whoever owns the work.
- Never delete plans in an automated sweep. The decision is made once per spec, by a
  person, after all three gates pass — including when the work "obviously shipped", which
  is precisely the case the gates exist for.
