# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository status

This repository is in **pre-implementation** state. Only `README.md`, `LICENSE`, and `.gitignore` exist — no Rust source, no `Cargo.toml`, no workspace yet. Treat early work here as bootstrapping: when adding code, you will need to initialize the Cargo workspace and the planned sub-crates yourself.

## Project intent

`airsstack` is the author's personal AI technology stack, written in Rust. The driving constraints (from `README.md`) shape every design decision:

- **Token efficiency over raw capability.** The author finds Claude Code too expensive due to token consumption. A primary objective of this stack is to *suppress token usage while preserving accuracy, reliability, and maintainability* — especially for software-engineering tasks. Favor designs that reduce tokens (caching, smaller models for sub-tasks, context pruning) over designs that maximize a single model's power.
- **Claude as the advanced model, OpenRouter for alternatives.** Claude models are the "advanced" tier; cheaper/alternative models (DeepSeek, Kimi K2, Qwen) are accessed via OpenRouter. The provider abstraction must accommodate this mixed-routing pattern naturally.
- **Replacement for Claude Code / OpenCode.** The CLI is meant as a personal substitute for those tools, not a general-purpose product. Design choices can be opinionated toward the author's workflow.

Inspirations called out in the README: LangChain, CrewAI, DSPy, DeepEval, BeeAI (frameworks); Claude Code, OpenCode, Gemini CLI (CLI agents). Use these as reference points when shaping APIs, but don't assume the author wants a clone of any one of them.

## Planned crate layout

The README lists these planned sub-crates (workspace members to be created):

- `airsstack-cli` — personal CLI agent, replacement for Claude Code / OpenCode
- `airsstack-core` — core agentic framework (the foundation the CLI is built on)
- `provider-claude` — Claude model provider
- `provider-openrouter` — OpenRouter provider (fronts DeepSeek, Kimi K2, Qwen, etc.)
- `airsdsp` — purpose not yet documented; confirm with the user before designing it

The `core` ↔ `provider-*` split implies providers are pluggable behind a trait owned by `core`. When you create that trait, keep the token-suppression objective in mind: the API should make it easy to route different parts of an agent loop to different models.

## Commands

No build/test/lint commands exist yet. Once the workspace is scaffolded, the standard Rust commands apply (`cargo build`, `cargo test`, `cargo test -p <crate>`, `cargo clippy`, `cargo fmt`). Add concrete invocations to this file as soon as the workspace is in place — especially any non-obvious ones (feature flags, integration-test setup, env vars for provider keys).

## Superpowers artifact paths (override defaults)

The `superpowers` plugin saves brainstorm specs and implementation plans by default under `docs/superpowers/`. In this repo, **override those defaults**:

- Brainstorm specs → `.superpowers/specs/YYYY-MM-DD-<topic>-design.md`
- Implementation plans → `.superpowers/plans/YYYY-MM-DD-<feature>.md`

The `.superpowers/` directory is **gitignored** — these artifacts are local-only scratch context, not committed documentation. Do not `git add` anything under `.superpowers/`. Do not propose moving them back under `docs/` or unignoring them. If a spec or plan contains decisions that belong in source control, copy the relevant decision into `CLAUDE.md`, a `.claude/rules/` file, or real docs under `docs/` — leave the generated artifact alone.

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
- `.claude/rules/git-commits.md` — Conventional Commits v1.0.0 with workspace-aware scopes (`fix(airsstack-core/...)`, `feat(airsstack-cli/...)`, `build(workspace): ...`, `docs(repo): ...`). Loads unconditionally.
