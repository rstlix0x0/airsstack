---
name: execute-plan
description: Use when you have a written implementation plan to carry out — executes it task by task with review checkpoints, driving the airsstack orchestrate skill per task and pausing for user review. Soft-coupled to the airsstack main plugin; if that plugin is absent, degrades to guided inline execution.
---

# Execute Plan

Drive a written plan to a reviewed, presentable state, task by task. This skill does not design and does not plan — those are `airsstack-sdd:brainstorm` and `airsstack-sdd:write-plan`. It works with any plan in a documented on-disk format, whether written by `write-plan` or by hand.

## Load and review

Read the plan fully before a line changes. If no explicit path was handed to you, the default location and naming are in `../../references/artifact-paths.md`.

Note each task, its acceptance criteria, and the verifications it specifies. Then assess it critically: ambiguous tasks, unclear dependencies, an unspecified branch, anything contradicting project conventions. Surface concerns NOW and resolve every blocking one — do not guess through an ambiguity and find the mistake three tasks later.

Once settled, create a `TodoWrite` list of every task. That list is the execution ledger.

## Safety guard

If the current branch is `main` or `master`, stop. Name the branch and get explicit consent before any implementation. Never execute a plan on a protected branch without the user saying so.

## Execution engine — soft coupling to `airsstack:orchestrate`

Drive each task through `airsstack:orchestrate`: it runs the coder → reviewer pipeline, handles the fix loop, and holds a per-task commit gate. Hand it one scoped task; it returns a reviewed result.

If it does not resolve — the `airsstack` main plugin is not installed — degrade to **guided inline execution** on the main thread, applying the same discipline the plan specifies (test first, confirm it fails, implement, run every verification the plan names). Tell the user the agent pipeline was unavailable. Never fail hard for want of the main plugin.

## Per-task loop

For each task in the ledger, in order:

1. Mark it `in_progress`.
2. Drive the implementation through `airsstack:orchestrate` (or inline), giving it the task description, acceptance criteria, and named verifications.
3. Run every verification the plan specifies for this task.
4. Pause and surface the result: which files changed and what behavior, each verification's outcome with evidence, and — when driven through orchestrate — the reviewer's own report, not a summary of it.
5. Only once it passes review and verification, mark it `completed`.

A task that fails verification is not complete; do not start the next one. Where the plan designates checkpoint boundaries ("pause for review after tasks 1–3"), treat them as hard stops: present the whole batch and wait for "continue".

## When to stop and ask

Stop immediately when a dependency is missing or ambiguous, a verification fails repeatedly in a way the plan does not explain, the plan has a gap where guessing would be risky, or an unexpected conflict appears (naming collision, changed API, a test suite in a state the plan did not anticipate).

Ask the one focused question that unblocks you — not every concern at once.

## Completion

Present the whole run: what was built task by task, the verifications and their evidence, the full reviewer report (or inline evidence, on the degraded path), and any deviation from the plan and how it was resolved.

Then wait. The user decides whether to commit, merge, or open a pull request. Do not auto-commit, do not auto-merge, do not auto-push — the commit gate belongs to the user.
