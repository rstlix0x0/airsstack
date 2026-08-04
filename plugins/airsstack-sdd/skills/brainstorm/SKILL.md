---
name: brainstorm
description: Use before any creative or build work — turns a rough idea into an approved design spec through a one-question-at-a-time design dialogue, then writes the spec to the SDD specs directory and hands off to the write-plan skill. Invoke at the very start, before writing code or scaffolding anything.
---

# Brainstorm

Transform a rough idea into a fully-formed, user-approved design spec through structured collaborative dialogue. The only goal of this skill is to produce a spec the user stands behind — understanding intent completely before any implementation begins.

## Hard gate

Do NOT invoke any implementation skill, write code, or scaffold anything until you have presented a complete design AND received explicit user approval. This rule holds regardless of how simple the work appears. A simple task may produce a short design, but the design must still be written, presented, and approved before moving forward.

## Checklist

Work through these steps in order. Create a `TodoWrite` item for each step so progress is visible.

1. **Explore project context.** Read relevant files, docs, and recent commits to understand the codebase, its conventions, and what already exists. Do not design in a vacuum. **Check for RFCs:** an engineer may have dropped an RFC into the SDD `rfcs/` directory (location in `../../references/artifact-paths.md`) as design input. If the user explicitly named an RFC — by path or filename — load it as primary design input; if that named file is missing, report the path and ask for a correct reference rather than guessing. Otherwise auto-scan `rfcs/` for any files: if it holds RFCs, surface them and ask which (if any) are relevant before proceeding; if it is empty or absent, proceed normally with no RFC prompt. RFCs are read-only input — never create, edit, move, or delete one. **Detect the active stack** and load its guideline now — see below.

2. **Assess scope.** Before refining any details, judge whether the request spans multiple independent subsystems. If it does, flag this immediately and help the user decompose it into separate, sequenced scopes — each one gets its own spec, plan, and implementation cycle. Do not proceed with a multi-scope design as though it were one unit of work.

3. **Ask clarifying questions one at a time.** Surface the questions that matter most for the design: purpose, constraints, success criteria, non-goals. Prefer multiple-choice questions where natural — they give the user concrete options and keep the dialogue moving. Never ask a battery of questions at once; ask one, get the answer, then ask the next.

4. **Propose 2–3 approaches.** Once you understand the intent well enough, present two or three distinct approaches along with their trade-offs. Lead with your recommendation and explain why you favor it. Invite the user to redirect before committing to any path.

5. **Present the design section by section.** Walk through the design in sections scaled to their complexity. At minimum, cover architecture, key components and their responsibilities, data flow, error handling, and testing strategy. Each section must conform to the active stack's guideline architecture rules loaded in step 1 — call out where a design choice is driven by a guideline rule. After each non-trivial section, confirm the user's understanding and agreement before moving to the next. This incremental gate catches disagreements early, before the full spec is written.

6. **Write the spec.** Once the design is agreed upon, write it to the SDD specs directory — its location and `YYYY-MM-DD-<topic>.md` naming are defined in `../../references/artifact-paths.md` (read it for the exact path). Before writing, ensure that directory exists, creating it if absent: the SessionStart hook or `/airsstack-sdd:setup` normally provisions it, but never assume a wrapper ran. The spec is the durable record — write it to stand on its own without reference to this conversation. **RFC provenance:** if one or more RFCs seeded this spec, record each in the spec header with a `Derived-from-RFC: rfcs/<filename>` line — one line per source RFC. Omit the line entirely when no RFC seeded the spec. Note that `rfcs/` is worktree-local while the spec is written to the HOME-global store, so a `Derived-from-RFC` pointer may reference a file that is absent when the spec is read from another worktree — this is expected; the line is provenance, not a live link.

7. **Self-review the spec.** Re-read what you wrote from the perspective of someone seeing it for the first time, and fix issues directly in the file. Check for: **placeholders** — no TBD, TODO, "to be determined," or vague deferral language; either fill the gap or make the decision explicit. **Internal consistency** — component names, data shapes, and behavioral descriptions agree throughout; a component described one way in the architecture section must match its description in the error-handling section. **Scope** — the spec is focused enough to map to a single plan and a coherent implementation cycle; if multiple independent objectives are woven together, decompose before proceeding. **Ambiguity** — wherever the spec could be read two ways, pick one interpretation and make it explicit; ambiguous specs produce divergent implementations.

8. **User review gate.** Ask the user to read the spec file you just wrote (path in `../../references/artifact-paths.md`) and give explicit approval. This is a mandatory stop, not a formality. If they request changes — small clarifications or significant redesigns — revise the spec and re-run the self-review. Proceed only after the user gives the go-ahead. Committing the spec is the user's call — do not auto-commit.

9. **Hand off.** Invoke `airsstack-sdd:write-plan` to convert the approved spec into an implementation plan. That is the only skill you invoke after this one and the only next step — do not write code or scaffold files yourself; the approved spec is the handoff artifact.

## Design for isolation

When structuring the design, break the system into small units each with one clear purpose and well-defined interfaces. Each unit should be understandable and testable on its own, without needing to hold the rest of the system in your head. Highly coupled designs are harder to test, harder to change, and harder to reason about in review. Reach for loose coupling and obvious interfaces over clever integration.

## Honor the active stack's guidelines

Before proposing architecture, detect the project's active stack(s) and load the matching guideline. A guideline plugin advertises itself with an `enforcement.json` at its root (read by the `airsstack` plugin's enforcement dispatcher); its `detect` markers — e.g. `Cargo.toml` for Rust — tell you which stack a repo is. For every active stack whose guideline skill is installed (e.g. `airsstack-guideline-rust:rust-guidelines`), invoke that skill and let its **architecture** rules — not merely its Definition of Done — shape the design: type modeling, module layout, dispatch choices, doc and test mandates. A spec that ignores the guideline's architecture rules produces a plan that bakes those violations in before a single line of code is written. If no installed guideline matches the active stack, say so and proceed on general principles.

## Key principles

The checklist carries the dialogue rules (one question at a time, multiple-choice where natural, 2–3 alternatives, agreement section by section). Two more bind throughout:

- **YAGNI ruthlessly.** No designing for imagined future requirements. Every component in the spec has a concrete, immediate reason to exist.
- **Be flexible.** If the user redirects mid-dialogue, update your understanding and carry forward without defending the prior path.
