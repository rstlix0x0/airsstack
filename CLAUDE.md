# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository status

The Cargo workspace exists (root `Cargo.toml`, `resolver = "3"`, Edition 2024). It has **two members**: `crates/clauders` (a Claude SDK crate) and `crates/openrouter-rs` (an OpenRouter SDK crate, scaffold only — no public API surface yet). Add new members under `crates/` per [[rust-workspace]] only when there is concrete work for them; do not pre-create speculative crates.

## Project intent

`airsstack` is the author's personal AI technology stack, written in Rust. The driving constraints (from `README.md`) shape every design decision:

- **Token efficiency over raw capability.** The author finds Claude Code too expensive due to token consumption. A primary objective of this stack is to *suppress token usage while preserving accuracy, reliability, and maintainability* — especially for software-engineering tasks. Favor designs that reduce tokens (caching, smaller models for sub-tasks, context pruning) over designs that maximize a single model's power.
- **Claude as the advanced model, OpenRouter for alternatives.** Claude models are the "advanced" tier; cheaper/alternative models (DeepSeek, Kimi K2, Qwen) reachable via OpenRouter are the longer-term vision for mixed routing. This is a *direction*, not current scope — see "Scope discipline" below.

Inspirations called out in the README: LangChain, CrewAI, DSPy, DeepEval, BeeAI (frameworks); Claude Code, OpenCode, Gemini CLI (CLI agents). Use these as reference points when shaping APIs, but don't assume the author wants a clone of any one of them.

## Scope discipline

Be pragmatic; do not build for an imagined future. The repo deliberately ships **only what there is concrete work for** — today that is the `clauders` crate and the `openrouter-rs` scaffold. Earlier planning named a fleet of crates (`airsstack-cli`, `airsstack-core`, `provider-claude`, `provider-openrouter`, `airsdsp`); those names are **obsolete — do not reintroduce, design, or reference them**. (The OpenRouter provider now lives in the `openrouter-rs` member, which supersedes the old `provider-openrouter` name; its surface is still to be scoped.) If the author decides to add a crate, it gets named and scoped at that point, and the workspace + commit-scope + `.superpowers` tier vocabulary are updated together (see [[rust-workspace]], [[git-commits]], [[ai-superpowers-artifacts]]).

## Commands

The workspace is in place; the standard Rust commands apply (`cargo build`, `cargo test`, `cargo test -p clauders`, `cargo clippy`, `cargo fmt`). The full pass/fail gate is the Definition of Done in [[rust-strict-quality]]. `clauders` is feature-gated (e.g. `transport-reqwest`); use `cargo hack --each-feature` for feature combinatorics per [[rust-strict-quality]].

## Superpowers artifact paths (override defaults)

The `superpowers` plugin saves brainstorm specs and implementation plans by default under `docs/superpowers/`. In this repo, **override those defaults** and tier them by crate / `workspace` / `repo` (full layout + plan-deletion lifecycle in [[ai-superpowers-artifacts]]):

- Brainstorm specs → `.superpowers/specs/<tier>/YYYY-MM-DD-<topic>.md`
- Implementation plans → `.superpowers/plans/<tier>/YYYY-MM-DD-<topic>.md`

The `.superpowers/` directory is **gitignored** — these artifacts are local-only scratch context, not committed documentation. Do not `git add` anything under `.superpowers/`. Do not propose moving them back under `docs/` or unignoring them. If a spec or plan contains decisions that belong in source control, copy the relevant decision into `CLAUDE.md`, a `.claude/rules/` file, or real docs under `docs/` — leave the generated artifact alone. Completed plans may be deleted (gitignored = irreversible, so confirm first) once their spec is the source of truth — see [[ai-superpowers-artifacts]].

## Project rules

Topic-specific rules live in `.claude/rules/` and are auto-discovered. Path-scoped via YAML frontmatter where useful (saves context). Current rules:

- `.claude/rules/rust-microsoft-guidelines.md` — Microsoft Pragmatic Rust Guidelines, scoped to `**/*.rs` and Cargo manifests.
- `.claude/rules/rust-strict-quality.md` — strict pass/fail bar for every Rust change: zero warnings (build + clippy + rustdoc), all tests green including doctests, defined Definition-of-Done command set.
- `.claude/rules/rust-workspace.md` — workspace layout, root vs member `Cargo.toml`, centralized `[workspace.package|dependencies|lints]`, naming, publishing order. Based on the official Cargo Book ch. 14.3.
- `.claude/rules/rust-static-dispatch.md` — prefer generics over `Box<dyn Trait>`; lists the narrow justified exceptions; clarifies that `Arc<Inner>` for cheap-`Clone` services is NOT a trait-object pattern and stays allowed (per `M-SERVICES-CLONE`).
- `.claude/rules/rust-strong-types.md` — no primitive obsession: newtype every domain string/int/bool, validate at construction (parse-don't-validate), type-state pattern for required-field builders and ordered lifecycles, no `bool` params for semantic flags.
- `.claude/rules/rust-mod-rs-export-only.md` — `mod.rs` / `lib.rs` are table-of-contents only: module docs + `mod` / `pub mod` / `pub use`. No struct/enum/trait/impl/fn/item-emitting macro. Implementation lives in sibling files named after the item.
- `.claude/rules/rust-doc-comment-discipline.md` — rustdoc and `//` comments target downstream engineers; no `.claude/` / `.superpowers/` paths, no plan/phase/task identifiers, no workflow vocabulary, no AI/agent names in source. Internal artifact references belong in commit messages and PR descriptions, not in shipped code.
- `.claude/rules/rust-unit-test-mandate.md` — every logic-bearing `src/*.rs` ships colocated `#[cfg(test)] mod tests`. Five structural exemptions (export-only / pure typedef / pure trait def / mockall-generated / build.rs+const-tables) must be cited inline. Integration tests under `tests/` complement but do not substitute. Deferrals require both an inline reason and a tracking reference.
- `.claude/rules/git-commits.md` — Conventional Commits v1.0.0 with workspace-aware scopes (`fix(clauders/...)`, `build(workspace): ...`, `docs(repo): ...`). Loads unconditionally.
- `.claude/rules/ai-model-routing.md` — model tier for delegated agents: Sonnet=execution/coding, Opus=think/analyze/review/debug/design, Haiku=narrow non-coding trivia only. Explicit `model:` mandatory on code/review/think agents (agentType/workflow defaults inherit a cheap tier). Governs `Agent` spawns + workflow `agent()`/`meta.phases[].model`; does not control the main loop. Loads unconditionally. (`ai-*` prefix = rules about how AI/agents operate, vs `rust-*` for code.)
- `.claude/rules/ai-agent-orchestration.md` — binding flow for delegated agents: agents are leaves (no Agent→Agent), the coder→code-reviewer→spec-reviewer→user-approval pipeline, findings route through the orchestrator, no agent commits, selective delegation, validate-before-trust. Loads unconditionally. Sibling to `ai-model-routing.md` (routing = which model; orchestration = how they chain).
- `.claude/rules/ai-superpowers-artifacts.md` — `.superpowers/` specs + plans are tiered by crate / `workspace` / `repo` (same vocabulary as commit scopes); specs are the durable record, plans are disposable scaffolding deletable once their spec is source-of-truth; `.superpowers/` is gitignored so deletion is irreversible — confirm first, prefer archiving when recall value is unclear. Loads unconditionally.

## Repo agents

Four repo-owned subagents live in `.claude/agents/` (governed by `ai-agent-orchestration.md` + `ai-model-routing.md`):

- `airsstack-coder` (sonnet) — implements one scoped task with strict TDD, runs the DoD, never commits.
- `airsstack-code-reviewer` (opus) — re-runs the DoD and reviews the diff against `.claude/rules/`; report-only.
- `airsstack-spec-reviewer` (opus) — reviews implementation against `.superpowers/` spec/plan intent; report-only.
- `airsstack-verifier` (opus) — audits the phase's accumulated claims (coder + reviewer receipts) against ground truth at the final gate; emits a VERIFIED/REFUTED/UNCONFIRMED ledger for the user. Report-only leaf, runs once per phase.

Prefer these over the generic `caveman:cavecrew-*` agents for Rust work — the cavecrew agents pin Haiku for review and cannot run the DoD. Spawn by name via `Agent` `subagent_type`, pinning `model:` per the routing rule. A newly-added agent file under `.claude/agents/` is only spawnable in a session that started with it present.

## Repo skills

Repo-local skills live under `.claude/skills/<name>/SKILL.md` (auto-discovered, `/`-invocable):

- `snapshot-save` — codifies the memory-save ceremony: judges durable session facts, writes/updates one file per fact under the existing memory schema (`user`/`feedback`/`project`/`reference`), updates `MEMORY.md`. Has a mandatory durability gate (thin session → writes nothing). "Snapshot" is just the author's term for the existing memory; it is NOT a new format.
- `snapshot-load [topic]` — codifies the memory-load ceremony: reads `MEMORY.md`, fully reads files relevant to the current git branch + optional topic arg, reports the rehydrated state.

Session-boundary hooks in `.claude/settings.json` nudge these automatically: `SessionStart` → `/snapshot-load`, `SessionEnd` → `/snapshot-save`. Hooks only nudge; the model (running the skill) does the judgment, including the durability gate. A skill/hook registers only in a session that starts with the file present.
