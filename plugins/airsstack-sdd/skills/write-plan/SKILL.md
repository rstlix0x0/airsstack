---
name: write-plan
description: Use when you have an approved spec and need an implementation plan — decomposes one objective into bite-sized test-first tasks with exact file paths and complete code, writes the plan to the SDD plans directory, and owns the spec-versus-plan artifact lifecycle. One objective per plan; read references/artifact-lifecycle.md before deleting any plan.
---

# Write Plan

Turn one approved-spec objective into a construction manual an implementer can execute with zero prior
knowledge of the codebase: exact file paths, complete code, runnable commands, expected output, and a
commit at the end of each task. Not aspirational prose.

A plan stands alone. Whoever picks it up may not know the codebase, may not have read the spec, and
shares no context with you — so every task carries everything that task needs.

Three principles bind the content:

- **TDD.** Every behavioral change is preceded by a failing test: write the failing test → confirm it
  fails → minimal code to pass → confirm it passes → commit. No task skips the red-green cycle.
- **Honor the active stack's guidelines.** Detect the stack from repo markers (`Cargo.toml` → Rust) and
  load the matching guideline skill (`airsstack-guideline-rust:rust-guidelines`). Every code block you
  write must already conform to its architecture rules, and each task's verification must include its
  Definition of Done. A plan that emits rule-violating code is a defect even if the code works.
- **DRY / YAGNI.** No abstraction the objective does not require. Extract shared code only where two
  tasks would genuinely duplicate it.

## Scope check — one objective per plan

A plan covers **exactly one objective**: one outcome stateable in a single sentence without an "and".
If the goal sentence needs an "and", split the plan. Tasks are not objectives — three tasks
implementing parts of one feature belong in one plan.

Sibling plans sharing a spec are disambiguated by topic, not number:
`2026-06-01-auth-token-validation.md`, not `2026-06-01-auth-plan-1.md`.

## File structure first

Before defining tasks, map the file changes, one sentence of responsibility each. Prefer a new focused
file over expanding an existing one.

```
src/auth/token.rs          — [create] token validation logic and unit tests
src/auth/mod.rs            — [modify] re-export the new token module
tests/auth_integration.rs  — [create] integration test for the token round-trip
```

Then assign each file to exactly the tasks that touch it. A task listing files it does not touch is a
defect; a file in no task is a dangling artefact.

## Task granularity

Each task is a 2–5 minute action — doable, testable, and committable in one sitting. Longer means too
coarse; break it down. "Write the implementation and tests for X" as one step is a collapsed red-green
cycle — expand it.

## Header template

```markdown
# [Feature Name] Implementation Plan

**Goal:** [one sentence — no "and" joining two distinct objectives]

**Architecture:** [2-3 sentences on the structural decisions this plan makes]

**Tech Stack:** [key technologies, libraries, frameworks]

---
```

The Goal line is the scope guard. If you cannot write it without "and", stop and split.

## Task template

Each task names its files, then walks the red-green cycle with real code and real expected output at
every step. Substitute your actual language and names:

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
   ```

2. Run it and confirm failure:

   ```
   $ pytest tests/test_add.py
   FAILED tests/test_add.py::test_add_two_positive_integers — NameError: name 'add' is not defined
   ```

3. Write the minimal implementation in `src/math/add.py`:

   ```python
   def add(a: int, b: int) -> int:
       return a + b
   ```

4. Run it and confirm green:

   ```
   $ pytest tests/test_add.py
   1 passed in 0.01s
   ```

5. Export from the module index, then commit `feat(math): add integer addition function`.
````

Where two tasks share code structure, write it out in full in both — a plan that says "similar to Task
N" is no longer standalone, which is the property the whole format exists to protect.

## No placeholders

Fix these before saving:

- `TBD`, `TODO`, `implement later`.
- "add appropriate error handling / validation / edge cases" without naming them and showing the code.
- "write tests for the above" without the test code.
- A step saying *what* without showing *how* — no code block, no command, no expected output.
- A reference to a type, function, or constant defined neither earlier in the plan nor in the codebase.

## Before saving

Check the draft on three axes, fixing inline:

- **Spec coverage** — every in-scope spec requirement maps to a task, or is explicitly deferred with a
  justification.
- **Type consistency** — every type, signature, and constant used in Task N+1 was defined in an earlier
  task or already exists. A forward reference is a defect: reorder, or add the definition.
- **Guideline conformance** — every code block scanned against the active guideline's architecture
  rules. Cheaper to fix here than after the coder ships it. If no guideline matches the stack, say so.

## Execution handoff

Save to the SDD plans directory — location and `YYYY-MM-DD-<topic>.md` naming are in
`../../references/artifact-paths.md`. Create the directory if absent; do not assume the provisioning
hook or `/airsstack-sdd:setup` has run.

Two execution paths:

1. **Subagent-driven.** A fresh subagent per task via `airsstack-sdd:execute-plan`. Review each receipt
   before spawning the next; independent tasks may run in parallel. Use for multi-file changes or to
   keep the main thread clear.
2. **Inline.** Execute in-session with a review checkpoint between tasks. Use for simpler objectives
   where delegation costs more than it saves.

## Artifact lifecycle

Specs are the durable record of intent; plans are disposable scaffolding derived from them. Once the
work has shipped and the spec reflects everything that changed during implementation, the plan becomes
a deletion candidate — but only after three gates pass. The gates, the irreversibility caveat, and the
anti-patterns live in `references/artifact-lifecycle.md`. Read it before deleting any plan.
