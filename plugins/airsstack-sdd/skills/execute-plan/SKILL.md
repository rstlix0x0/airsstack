---
name: execute-plan
description: Use when you have a written implementation plan to carry out — executes it task by task with review checkpoints, driving the airsstack orchestrate skill per task and pausing for user review. Soft-coupled to the airsstack main plugin; if that plugin is absent, degrades to guided inline execution.
---

# Execute Plan

Carry out a written implementation plan from start to finish — task by task, with a review checkpoint after each task and a final user decision at the end. The only job of this skill is to drive planned work to a reviewed, presentable state. It does not design, it does not plan; those belong to `airsstack-sdd:brainstorm` and `airsstack-sdd:write-plan`.

This skill works with any plan that follows a documented on-disk format, whether produced by `airsstack-sdd:write-plan` or written by hand.

## Load and review

Before a single line changes, read and understand the plan fully. If the caller did not hand you an explicit plan path, the default plan location and naming are defined in `../../references/artifact-paths.md` — look there.

1. Read the plan file from disk. Note each task, its description, its acceptance criteria, and the verifications the plan specifies.
2. Assess the plan critically. Ask yourself: Are any tasks ambiguous? Are dependencies between tasks clear? Is the branch specified, or does one need to be created? Does anything in the plan contradict known project conventions?
3. If you find concerns or open questions, surface them to the user NOW — before any work starts. Resolve every blocking question before proceeding. Do not silently guess through an ambiguity and discover the mistake three tasks later.
4. Once the plan is understood and any concerns are settled, create a `TodoWrite` list with every task. This list is the execution ledger: it makes progress visible and prevents tasks from being skipped.

## Safety guard

If the current branch is `main` or `master`, stop immediately. Tell the user which branch you are on and ask for explicit consent before starting any implementation. Never execute a plan directly on a protected branch without the user saying so.

## Execution engine — soft coupling to `airsstack:orchestrate`

For each task, the preferred execution path is to drive it through the `airsstack:orchestrate` skill. That skill runs the full coder → reviewer pipeline, handles the fix loop, and holds a per-task commit gate. You hand it one scoped task at a time; it returns a reviewed result.

If `airsstack:orchestrate` does not resolve — because the `airsstack` main plugin is not installed — degrade gracefully to **guided inline execution**:

- Follow the plan's test-first steps directly on the main thread.
- Run every verification the plan specifies (build, test, lint, or whatever the plan names).
- Apply the same discipline: write the test first, confirm it fails, implement until it passes, run the full verification pass.
- Tell the user clearly that the agent pipeline was unavailable and that you are executing inline.

Never fail hard for want of the main plugin. The degraded path is slower and less isolated, but it produces a correct result when guided carefully.

## Per-task loop

Execute each task in the `TodoWrite` list in order. For each:

1. Mark the task `in_progress`.
2. Drive the implementation through `airsstack:orchestrate` (or inline, if degraded). Provide the orchestrate skill with the task description, its acceptance criteria, and the verifications the plan names.
3. Run every verification the plan specifies for this task.
4. Pause and surface the result to the user before moving on: what changed (which files, what behavior), each verification's outcome with evidence, and — when driven through `airsstack:orchestrate` — the reviewer's report from inside that skill, so the user sees the full picture rather than a summary of it.
5. Only when the task passes its review and verification, mark it `completed` and move to the next.

Do not start the next task until the current one is marked complete. A task that fails verification is not complete. If the plan designates explicit checkpoint boundaries (for example, "pause for user review after tasks 1–3"), honor those as hard stops: present all accumulated results for that batch and wait for the user to say "continue" before proceeding.

## When to stop and ask

Stop immediately and ask the user whenever:

- A task cannot start because a dependency is missing or ambiguous.
- A verification fails repeatedly and the failure is not explained by the plan.
- The plan contains a gap serious enough that guessing the right approach would be risky.
- An unexpected conflict arises — a naming collision, a changed API surface, a test suite in a state the plan did not anticipate.

Ask one focused question that unblocks you. Do not enumerate all possible concerns in one message; raise the one that is stopping you right now, get an answer, and continue.

## Completion

When every task has passed, present the whole run — what was built or changed task by task, the plan verifications and their evidence, the full reviewer report (or the inline evidence, on the degraded path), and any deviation from the plan and how it was resolved.

Then wait. The user decides whether to commit, merge, or open a pull request. Do not auto-commit, do not auto-merge, and do not auto-push — the commit gate belongs to the user.
