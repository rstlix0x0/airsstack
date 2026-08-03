# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Never assert what you have not checked

Every factual claim about the official SDKs, the `claude` binary, or this codebase must come from
something you opened in the current task. Not memory, not inference, not plausibility.

- **Cite it or drop it.** A behavioural claim carries a `file:line` or byte offset you actually read.
  If you cannot cite it, go read it or write "not verified" — never state it flatly.
- **A subagent's recommendation is not a fact.** Research agents mix findings with advice. Promoting
  "you should gate on X" into "the SDK gates on X" is fabrication even though a report said it.
  Verify the underlying claim yourself before it enters a spec, a doc, or code.
- **Absence needs a search that could have found it.** "X does not exist" names the exact command run
  and why it would have hit. Confirm the method works by finding a sibling you know is present.
- **A passing test proves nothing until you have seen it fail.** Break the fix, watch it go red.
- **Prefer the shipped artifact** — `sdk.d.ts`/`sdk.mjs`, the Python sdist, the binary — over
  documentation about them, and documentation over recollection.

This has produced real defects: wrong claims reached committed docs, and a workstream was scoped out
entirely on an unverified negative.

## How to answer

Answer the question asked, then stop. Default to a few sentences.

Lead with the outcome: the first sentence says what happened or what you found; detail follows for
whoever wants it.

Say the work, then the label — "the Agent SDK argv builder (Phase 3)", not "P3". Plain words over
house vocabulary: surface, substrate, seam, axis, cohesion, load-bearing, grounded.

Reach for a table, tree, or ASCII diagram when the content is structural: 3+ things compared on the
same axes, a pipeline, a file layout, a before/after size. Prose for everything else — a box around a
single fact costs more than the sentence it replaced.

Write at full precision, never compressed, for exact error text, shell commands, code, wire formats,
security warnings, and irreversible actions.

One topic per reply. Raise what blocks the current request; hold the rest until asked. A full backlog
is for when the author asks "what is left".

## Project

`airsstack` is the author's personal AI technology stack, written in Rust. Cargo workspace,
`resolver = "3"`, Edition 2024, three members:

- `crates/clauders` — Claude SDK. The driving objective is **100% feature parity and behavioral
  compatibility with Anthropic's official SDKs** across three pillars: the Messages API, the Agent
  SDK (drives the `claude` Code CLI as a subprocess), and Managed Agents (server-hosted stateful
  agents). A Rust caller gets what a Python or TypeScript caller gets, with idiomatic Rust
  ergonomics. Authoritative statement:
  [`crates/clauders/docs/vision-and-strategy.md`](crates/clauders/docs/vision-and-strategy.md).
- `crates/openrouter-rs` — an **independent** standalone OpenRouter SDK. Its former
  `OpenRouterRuntime`/`RoutingRuntime` integration into `clauders` was severed per vision §9.1; the
  crate itself is kept.
- `crates/airs-transport` — generic async transport with an HTTP/reqwest layer, shared by both SDK
  crates.

Add members under `crates/` only when there is concrete work for them. Be pragmatic; the repo ships
only what serves the parity target.

### Do not reintroduce

Removed at the parity pivot (vision §5) because none of it exists in the official SDKs: `ApiRuntime`
(the native Messages loop), cross-provider routing, the middleware/evals/orchestration framework
tier, and the native permission/judge/subagent/session engines. Obsolete crate names:
`airsstack-cli`, `airsstack-core`, `provider-claude`, `provider-openrouter`, `airsdsp`.

The token-efficiency / mixed-routing thesis (route sub-tasks to cheaper non-Claude models via
OpenRouter) is **shelved, not abandoned** — it returns only under vision §8, once all three pillars
are at parity. Do not design for it now.

Re-introducing any of the above is named and scoped by the author at that point, under vision §8.

## Commands

Standard Rust commands apply (`cargo build`, `cargo clippy`, `cargo fmt`). **The workspace is
featureless** — no crate declares any Cargo `[features]`, every module compiles unconditionally, and
the `mockall` test doubles live in consumer-owned dev-only `test_support` modules rather than behind
a feature. So `--all-features` is a no-op that equals the default build.

The pass/fail gate (Definition of Done) lives in the `airsstack-guideline-rust` plugin. Invoke that
skill for the command set rather than reconstructing it here.

## AI methodology — the airsstack plugin suite

The methodology ships as a Claude Code plugin suite from the in-repo marketplace
(`.claude-plugin/marketplace.json`), not as loose `.claude/rules/` files or repo-local agents. The
Rust rules, commit convention, model-routing, and agent-orchestration policies are delivered as
plugin skills and references — invoke the relevant skill rather than expecting always-on rule files.

| Plugin | What it provides |
|---|---|
| `airsstack` | coder, reviewer, explorer; orchestration driver; process guidelines; project-local snapshot memory; concise output mode |
| `airsstack-sdd` | spec-driven workflow: `brainstorm` → `write-plan` → `execute-plan` |
| `airsstack-guideline-rust` | Rust engineering guidelines and the Definition of Done |
| `airsstack-journal` | Obsidian-compatible journal vault kept outside the repo, written by isolated subagents |
| `airsstack-okf` | Open Knowledge Format v0.1 toolkit over the in-repo `knowledge/` bundle |
| `airsstack-cmux` | native cmux terminal control (control / workspace / browser / config) |
| `airsstack-plugin-dev` | cache-sync hook for developing the suite itself |

Install with `/plugin marketplace add .`, then `/plugin install <name>@airsstack` per plugin. Each
ships a README under `plugins/<name>/README.md`.

## Before re-deriving, query the stores

Both are index-first and token-cheap. Reach for them instead of re-deriving.

- **`/journal-recall <topic>`** — why something was built the way it is, what was already tried, what
  a past session decided. Returns ranked pointers from a derived index; open at most the one or two
  it surfaces. Mark a note that actually helped with `/journal-helped <stem>`.
- **`/okf-recall <question>`** — per-module reference for the Rust crates, cited back to source.
  Prefer it over re-reading source where the bundle covers the area. It is regenerated on demand
  (`/okf-enrich`), so it can lag: if a concept contradicts the code, trust the code and note the
  drift.
- **`/snapshot-load`** — per-branch session orientation ("where was I on this branch"), not durable
  history or reference knowledge.

## Conventions owned by the repo

- **Commits** follow Conventional Commits v1.0.0 with workspace-aware scopes: a crate name
  (`clauders`, `openrouter-rs`, `airs-transport`), `workspace` (root Cargo files / top-level config),
  or `repo` (`.claude/`, `.github/`, `plugins/`, `knowledge/`, docs). Full convention ships in the
  `airsstack` plugin.
- `.claude/settings.json` carries non-secret project settings; `.claude/settings.local.json` carries
  machine-local permission grants (gitignored).
