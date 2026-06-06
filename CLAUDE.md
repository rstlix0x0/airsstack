# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository status

The Cargo workspace exists (root `Cargo.toml`, `resolver = "3"`, Edition 2024). It has **two members**: `crates/clauders` (a Claude SDK crate) and `crates/openrouter-rs` (an OpenRouter SDK crate). Add new members under `crates/` only when there is concrete work for them; do not pre-create speculative crates.

## Project intent

`airsstack` is the author's personal AI technology stack, written in Rust. The driving constraints (from `README.md`) shape every design decision:

- **Token efficiency over raw capability.** The author finds Claude Code too expensive due to token consumption. A primary objective of this stack is to *suppress token usage while preserving accuracy, reliability, and maintainability* — especially for software-engineering tasks. Favor designs that reduce tokens (caching, smaller models for sub-tasks, context pruning) over designs that maximize a single model's power.
- **Claude as the advanced model, OpenRouter for alternatives.** Claude models are the "advanced" tier; cheaper/alternative models (DeepSeek, Kimi K2, Qwen) reachable via OpenRouter are the longer-term vision for mixed routing. This is a *direction*, not current scope — see "Scope discipline" below.

Inspirations called out in the README: LangChain, CrewAI, DSPy, DeepEval, BeeAI (frameworks); Claude Code, OpenCode, Gemini CLI (CLI agents). Use these as reference points when shaping APIs, but don't assume the author wants a clone of any one of them.

## Scope discipline

Be pragmatic; do not build for an imagined future. The repo deliberately ships **only what there is concrete work for** — today that is the `clauders` crate and the `openrouter-rs` crate. Earlier planning named a fleet of crates (`airsstack-cli`, `airsstack-core`, `provider-claude`, `provider-openrouter`, `airsdsp`); those names are **obsolete — do not reintroduce, design, or reference them**. If the author decides to add a crate, it gets named and scoped at that point.

## Commands

The standard Rust commands apply (`cargo build`, `cargo clippy`, `cargo fmt`). **Tests must run with `--all-features`**: `cargo test --workspace --all-features` for the full gate, or `cargo test -p <crate> --all-features` while iterating on one crate. Plain `cargo test` / `cargo test -p <crate>` compiles only the default features and **silently skips feature-gated tests** (e.g. the `__test-mocks` mock/integration tests), so a green default-feature run is NOT a valid gate. The crates are feature-gated (e.g. `transport-reqwest`, `__test-mocks`); use `cargo hack check --each-feature` for compile-time feature combinatorics. The full pass/fail gate (Definition of Done) lives in the `airsstack-guideline-rust` plugin — see below.

## AI methodology — the airsstack plugin suite

This repo's AI development methodology (execution agents, spec-driven workflow, Rust guidelines, memory, orchestration) is packaged as a **Claude Code plugin suite**, not as loose `.claude/rules/` files or repo-local agents. The marketplace and plugins live in this repo:

- `.claude-plugin/marketplace.json` — the `airsstack` marketplace.
- `plugins/airsstack/` — execution engine: a TDD coder, a merged code+spec reviewer, a claim verifier, a read-only explorer, an orchestration driver, process guidelines, project-local memory, and a concise output mode.
- `plugins/airsstack-sdd/` — spec-driven workflow: `brainstorm` → `write-plan` → `execute-plan`.
- `plugins/airsstack-guideline-rust/` — Rust engineering guidelines and the Definition-of-Done, delivered as a lazily-loaded skill.

To use the suite, install it from the in-repo marketplace:

```
/plugin marketplace add .
/plugin install airsstack@airsstack
/plugin install airsstack-sdd@airsstack
/plugin install airsstack-guideline-rust@airsstack
```

Each plugin ships its own README under `plugins/<name>/README.md`. The Rust rules, commit convention, model-routing, agent-orchestration, and superpowers-artifact policies that previously lived in `.claude/rules/` are now delivered as plugin skills/references — invoke the relevant skill (e.g. the Rust guideline) rather than expecting always-on rule files.

## Conventions still owned by the repo

- **Commits** follow Conventional Commits v1.0.0 with workspace-aware scopes: crate name (`clauders`, `openrouter-rs`), `workspace` (root Cargo files / top-level config), or `repo` (`.claude/`, `.github/`, `plugins/`, docs). Full convention ships in the `airsstack` plugin (`conventional-commits` guideline).
- `.claude/settings.json` carries non-secret project settings; `.claude/settings.local.json` carries machine-local permission grants (gitignored).
