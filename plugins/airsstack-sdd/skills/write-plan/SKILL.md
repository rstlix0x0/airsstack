---
name: write-plan
description: Use when you have an approved spec and need an implementation plan — decomposes one objective into bite-sized test-first tasks with exact file paths and complete code, writes the plan to the SDD plans directory, and owns the spec-versus-plan artifact lifecycle. One objective per plan; read references/artifact-lifecycle.md before deleting any plan.
---

# Write Plan

Given an approved spec, this skill turns one objective into a detailed, test-first implementation plan
that an implementer can execute with zero prior knowledge of the codebase. The plan is not aspirational
prose — it is a step-by-step construction manual: exact file paths, complete code, runnable commands,
expected outputs, and a commit at the end of each task.

## Overview

A plan must stand alone. The implementer who picks it up may not know the codebase, may not have read
the spec in detail, and may not share context with whoever wrote it. So every task in the plan carries
everything that task needs: the exact files to touch, the exact code to write (no stubs, no "add the
logic here"), the exact command to run, and the exact output that signals success.

Follow these principles throughout:

- **DRY / YAGNI.** Do not introduce abstractions the objective does not require. Extract shared code
  when two tasks would genuinely duplicate it; leave everything else concrete.
- **TDD.** Every behavioral change is preceded by a failing test. The sequence is always: write the
  failing test → confirm it fails → write minimal code to pass → confirm it passes → commit.
- **Frequent commits.** Each task ends with a commit. Small commits are easier to bisect, review, and
  revert; large commits are a planning smell.
- **Small, focused files.** Prefer many focused files over a few large ones. If a file is acquiring
  multiple responsibilities, that is a signal to split, not to keep stacking.
- **Honor the active stack's guidelines.** Detect the project's active stack from repo markers
  (e.g. `Cargo.toml` → Rust) and load the matching guideline skill (e.g.
  `airsstack-guideline-rust:rust-guidelines`). Every code block you write into a task must already
  conform to that guideline's architecture rules — strong types over primitives, table-of-contents
  module layout, static over dynamic dispatch, doc and test mandates — and each task's verification
  must include the guideline's Definition of Done. A plan that emits rule-violating code is a
  defect, even if the code "works."

## Scope check — one objective per plan

A plan file covers **exactly one objective**: one coherent outcome that can be stated in a single
sentence without an "and". If your goal sentence needs an "and," you have more than one objective —
split the plan.

A spec that spans multiple objectives produces multiple plan files, one per objective. Each plan file
is independently completable, reviewable, and deletable. Tasks inside a plan are not objectives — three
tasks that each implement part of the same feature still belong in one plan, because they serve one goal.

Sibling plans that share a spec should be disambiguated by topic in the filename, not by number alone:
`2026-06-01-auth-token-validation.md` and `2026-06-01-auth-session-management.md` tell you what each
covers; `2026-06-01-auth-plan-1.md` and `2026-06-01-auth-plan-2.md` do not.

Full lifecycle conventions — including the three gates that must pass before a plan may be deleted —
live in `references/artifact-lifecycle.md`. Read it before deleting any plan.

## File structure first

Before defining tasks, map the file changes the objective requires. For each file, state one sentence
describing its single responsibility. Prefer creating a new focused file over expanding an existing one.

```
Files changed by this objective:

src/auth/token.rs          — [create] token validation logic and unit tests
src/auth/mod.rs            — [modify] re-export the new token module
tests/auth_integration.rs  — [create] integration test for the token round-trip
```

Once the file map is complete, assign each file to exactly the tasks that need it. A task that lists
files it does not actually touch is a plan defect; a file that appears in no task is a dangling artefact.

## Bite-sized task granularity

Each task is a 2–5 minute action — a unit of work that can be done, tested, and committed in one
sitting without consulting anyone. A task that takes longer than that is too coarse; break it down.

The canonical task sequence is:

1. Write the failing test.
2. Run the test; confirm it fails (show the expected failure output).
3. Write the minimal implementation code to make it pass.
4. Run the test; confirm it passes (show the expected passing output).
5. Commit.

No task skips the red-green cycle. If you find yourself writing "write the implementation and tests for
X" as a single step, you have collapsed the cycle — expand it.

## Plan document header template

Every plan file must start with this header block:

```markdown
# [Feature Name] Implementation Plan

**Goal:** [one sentence — must not contain "and" joining two distinct objectives]

**Architecture:** [2-3 sentences describing the structural decisions this plan makes]

**Tech Stack:** [key technologies, libraries, or frameworks involved]

---
```

The Goal line is the plan's scope guard. If you cannot write it without "and," stop and split the plan.

## Task structure template

Each task follows this skeleton. The code example below uses a language-neutral `add` function to
demonstrate the template shape — substitute your actual language, types, and names when authoring a
real plan:

````markdown
### Task N — [Short imperative title]

**Files:**
- Create `src/math/add.py`
- Modify `src/math/__init__.py`
- Test `tests/test_add.py`

**Steps:**

1. Write the failing test in `tests/test_add.py`:

   ```python
   def test_add_two_positive_integers():
       assert add(2, 3) == 5

   def test_add_with_zero():
       assert add(0, 7) == 7
   ```

2. Run the test suite and confirm failure:

   ```
   $ pytest tests/test_add.py
   FAILED tests/test_add.py::test_add_two_positive_integers — NameError: name 'add' is not defined
   ```

3. Write the minimal implementation in `src/math/add.py`:

   ```python
   def add(a: int, b: int) -> int:
       return a + b
   ```

4. Run the test suite and confirm green:

   ```
   $ pytest tests/test_add.py
   2 passed in 0.01s
   ```

5. Export the new function from the module index:

   ```python
   # src/math/__init__.py
   from .add import add
   ```

6. Commit:

   ```
   feat(math): add integer addition function
   ```
````

Repeat this skeleton for every task. If two tasks share code structure, write it out in full in both —
never write "similar to Task N" and omit the code.

## No placeholders

A plan that contains any of the following patterns is incomplete. Fix them before saving:

- `TBD`, `TODO`, or `implement later` anywhere in the plan.
- "add appropriate error handling / validation / edge cases" without specifying what those are and
  showing the code.
- "write tests for the above" without the actual test code in the plan.
- "similar to Task N" as a substitute for repeating the code — repeat it.
- A step that says *what* to do without showing *how* (no code block, no command, no expected output).
- A reference to a type, function, or constant that has not yet been defined anywhere in the plan or
  the existing codebase.

## Self-review

After drafting the plan, run through these four passes before saving:

1. **Spec-coverage pass.** List every requirement in the spec's scope that this plan addresses. For
   each requirement, identify which task satisfies it. If any requirement maps to no task, the plan has
   a gap — add the task or note explicitly that the requirement is deferred (with justification).

2. **Placeholder scan.** Search the draft for the patterns listed in the "No placeholders" section.
   Fix every hit inline. The plan goes to the SDD plans directory (see `../../references/artifact-paths.md`) only once the scan is clean.

3. **Type-consistency check.** Verify that every type name, function signature, and constant referenced
   in Task N+1 was either defined in a previous task or already exists in the codebase. A forward
   reference to something the plan has not yet created is a defect. Resolve it by reordering tasks or
   by adding the missing definition.

4. **Guideline-conformance pass.** For each active stack, re-read the guideline's architecture rules
   and scan every code block in the plan against them — strong types over primitives, table-of-contents
   modules, static over dynamic dispatch, the doc and unit-test mandates. Fix any violation in the
   plan now; it is far cheaper to correct in the plan than after the coder has shipped it. If no
   installed guideline matches the stack, note that and rely on general principles.

Fix all findings inline before moving on. The self-review is not optional.

## Execution handoff

Save the plan to the SDD plans directory — location and `YYYY-MM-DD-<topic>.md` naming are defined in `../../references/artifact-paths.md`. Before writing, ensure that directory exists, creating it if absent; do not assume the provisioning hook or `/airsstack-sdd:setup` has run.

Two execution paths are available — choose based on task complexity and how much main-context you want
to preserve:

1. **Subagent-driven.** Spawn a fresh subagent per task using the `airsstack-sdd:execute-plan` skill.
   Each subagent receives the task brief, runs it, and returns a receipt. Review each receipt before
   spawning the next subagent. Independent tasks may run in parallel. Use this path for multi-file
   changes or when you want to keep the main thread clear.

2. **Inline.** Execute each task directly in-session, pausing for a review checkpoint between tasks.
   Use this path for simpler objectives or when delegating would cost more overhead than it saves.

Point the implementer (or yourself) to `airsstack-sdd:execute-plan` for the subagent path.

## Artifact lifecycle

Specs are the durable record of intent; plans are disposable scaffolding derived from them. Once the
work described by a plan has shipped and the spec has been updated to reflect everything that changed
during implementation, the plan is a deletion candidate — but not before three gates pass.

The full conventions — including the gated deletion lifecycle, the irreversibility caveat, and the
anti-patterns to avoid — live in `references/artifact-lifecycle.md`. Read it before deleting any plan.
