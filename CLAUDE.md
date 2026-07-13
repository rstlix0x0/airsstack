# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository status

The Cargo workspace exists (root `Cargo.toml`, `resolver = "3"`, Edition 2024). It has **three members**: `crates/clauders` (a Claude SDK crate), `crates/openrouter-rs` (an OpenRouter SDK crate), and `crates/airs-transport` (a generic async transport substrate with an HTTP/reqwest layer, shared by the two SDK crates). Add new members under `crates/` only when there is concrete work for them; do not pre-create speculative crates.

## Project intent

`airsstack` is the author's personal AI technology stack, written in Rust. The driving constraints (from `README.md`) shape every design decision:

- **Token efficiency over raw capability.** The author finds Claude Code too expensive due to token consumption. A primary objective of this stack is to *suppress token usage while preserving accuracy, reliability, and maintainability* — especially for software-engineering tasks. Favor designs that reduce tokens (caching, smaller models for sub-tasks, context pruning) over designs that maximize a single model's power.
- **Claude as the advanced model, OpenRouter for alternatives.** Claude models are the "advanced" tier; cheaper/alternative models (DeepSeek, Kimi K2, Qwen) reachable via OpenRouter are the longer-term vision for mixed routing. This is a *direction*, not current scope — see "Scope discipline" below.

Inspirations called out in the README: LangChain, CrewAI, DSPy, DeepEval, BeeAI (frameworks); Claude Code, OpenCode, Gemini CLI (CLI agents). Use these as reference points when shaping APIs, but don't assume the author wants a clone of any one of them.

## Scope discipline

Be pragmatic; do not build for an imagined future. The repo deliberately ships **only what there is concrete work for** — today that is the `clauders` crate and the `openrouter-rs` crate. Earlier planning named a fleet of crates (`airsstack-cli`, `airsstack-core`, `provider-claude`, `provider-openrouter`, `airsdsp`); those names are **obsolete — do not reintroduce, design, or reference them**. If the author decides to add a crate, it gets named and scoped at that point.

## Commands

The standard Rust commands apply (`cargo build`, `cargo clippy`, `cargo fmt`). **The workspace is featureless** — no crate declares any Cargo `[features]`, every module compiles unconditionally, and the `mockall` test doubles live in consumer-owned dev-only `test_support` modules rather than behind a feature. So `--all-features` is a harmless no-op that equals the default build, and plain `cargo test` compiles everything. The Definition of Done nonetheless standardizes on the explicit `--all-features` form for forward-safety: `cargo test --workspace --all-features` (full gate) or `cargo test -p <crate> --all-features` (single crate). The full pass/fail gate (Definition of Done) lives in the `airsstack-guideline-rust` plugin — see below.

## AI methodology — the airsstack plugin suite

This repo's AI development methodology (execution agents, spec-driven workflow, Rust guidelines, memory, knowledge, orchestration) is packaged as a **Claude Code plugin suite**, not as loose `.claude/rules/` files or repo-local agents. The marketplace and plugins live in this repo. Seven plugins ship from the in-repo marketplace:

- `.claude-plugin/marketplace.json` — the `airsstack` marketplace.
- `plugins/airsstack/` — execution engine: a TDD coder, a merged code+spec reviewer, a claim verifier, a read-only explorer, an orchestration driver, process guidelines, project-local snapshot memory, and a concise output mode.
- `plugins/airsstack-sdd/` — spec-driven workflow: `brainstorm` → `write-plan` → `execute-plan`.
- `plugins/airsstack-guideline-rust/` — Rust engineering guidelines and the Definition-of-Done, delivered as a lazily-loaded skill.
- `plugins/airsstack-journal/` — note-based experiential memory: an Obsidian-compatible journal vault written by isolated subagents (capture / note / recall / review / link / helped skills) with a derived recall index, kept outside the repo.
- `plugins/airsstack-okf/` — Open Knowledge Format (OKF) v0.1 producer+consumer toolkit: bundle provisioning, single-concept authoring, batch enrichment, progressive-disclosure recall, and a conformance linter over the in-repo `knowledge/` bundle.
- `plugins/airsstack-cmux/` — native cmux terminal control: four lazily-loaded skills (control / workspace / browser / config) that drive the cmux terminal.
- `plugins/airsstack-plugin-dev/` — plugin-development toolkit for the suite: a PostToolUse cache-sync hook that mirrors edited plugin files into the install cache.

To use the suite, install it from the in-repo marketplace:

```
/plugin marketplace add .
/plugin install airsstack@airsstack
/plugin install airsstack-sdd@airsstack
/plugin install airsstack-guideline-rust@airsstack
/plugin install airsstack-journal@airsstack
/plugin install airsstack-okf@airsstack
/plugin install airsstack-cmux@airsstack
/plugin install airsstack-plugin-dev@airsstack
```

Each plugin ships its own README under `plugins/<name>/README.md`. The Rust rules, commit convention, model-routing, agent-orchestration, and superpowers-artifact policies that previously lived in `.claude/rules/` are now delivered as plugin skills/references — invoke the relevant skill (e.g. the Rust guideline) rather than expecting always-on rule files.

## Project memory & knowledge — consult before re-deriving

Two in-suite stores hold context the source and git history don't, and both are **token-cheap to query** (index-first, ranked pointers, capped reads). Reach for them proactively rather than re-deriving — it serves the token thesis directly:

- **Development history → the journal (`airsstack-journal`).** An out-of-repo Obsidian vault of grounded session stories, decisions, and insights. When you need to know *why* something was built the way it is, what was already tried, or what a past session decided, run **`/journal-recall <topic>`** — it reads only the derived index and returns ranked pointers; open at most the one or two notes it surfaces. If a recalled note actually helped the task, mark it with **`/journal-helped <stem>`**. This is the "what happened and why" layer that `git blame` cannot answer.
- **Crate technical knowledge → the OKF bundle (`airsstack-okf`).** The in-repo `knowledge/` bundle holds per-asset OKF concept documents (roughly one per module/type) for the Rust crates, cross-linked and cited back to source. When you need a detailed reference for a specific crate module/type without reading the whole source tree, run **`/okf-recall <question>`** — it reads `index.md` first and opens only the concepts needed (progressive disclosure, hard-capped). Prefer this over re-reading source when the bundle covers the area.

These differ from the `airsstack` plugin's **snapshot memory** (`/snapshot-load`), which is per-branch *session orientation* ("where was I on this branch") — not durable history or reference knowledge. Use snapshots to resume, the journal to recall decisions, the OKF bundle to look up crate internals.

**Drift caveat:** the OKF bundle is regenerated on demand (`/okf-enrich`), not automatically, so it can lag the source and some areas may be unmapped. If a concept contradicts the current code, trust the code and note the drift.

## Conventions still owned by the repo

- **Commits** follow Conventional Commits v1.0.0 with workspace-aware scopes: crate name (`clauders`, `openrouter-rs`, `airs-transport`), `workspace` (root Cargo files / top-level config), or `repo` (`.claude/`, `.github/`, `plugins/`, `knowledge/`, docs). Full convention ships in the `airsstack` plugin (`conventional-commits` guideline).
- `.claude/settings.json` carries non-secret project settings; `.claude/settings.local.json` carries machine-local permission grants (gitignored).
