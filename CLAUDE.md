# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Never assert what you have not checked

Every factual claim about the official SDKs, the `claude` binary, or this codebase must come from
**something you opened in the current task**. Not memory, not inference, not plausibility.

- **Cite it or drop it.** A claim about behaviour carries a `file:line` or byte offset you actually
  read. If you cannot cite it, either go read it or write "not verified" — never state it flatly.
- **A subagent's recommendation is not a fact.** Research agents mix findings with advice. Promoting
  "you should gate on X" into "the SDK gates on X" is fabrication even though a report said it.
  Verify the underlying claim yourself before it enters a spec, a doc, or code.
- **Absence needs a search that could have found it.** "X does not exist" requires naming the exact
  command run and why it would have hit. Wrong identifier spelling, wrong file, wrong transport, or a
  summary instead of the artifact all prove nothing. Check the method works by confirming it finds a
  sibling you know is present.
- **A passing test proves nothing until you have seen it fail.** Before claiming a test covers a bug,
  break the fix and watch it go red.
- **Prefer the shipped artifact over documentation**, and documentation over recollection. Read
  `sdk.d.ts`/`sdk.mjs`, the Python sdist, or the binary — not what the docs say about them.

This has produced real defects repeatedly: wrong claims reached committed docs, and a workstream was
scoped out entirely on an unverified negative. When in doubt, go and look.

## How to talk to the author

**No jargon. Plain language. Short answers.**

- Explain things the way you would to a competent engineer who has not been staring at this
  problem for the last hour. Assume knowledge of Rust and software engineering; assume **no**
  memory of this repo's internal shorthand.
- **Never use an internal label without saying what it means**, every time: `WS 9`, `P1`, `S2`,
  `pillar`, `parity line`, `the epic`, `the tail`. These mean nothing on their own. Say what the
  work *is*, then the label if it is still useful.
- Ban filler vocabulary that adds no information: "surface", "substrate", "seam", "axis",
  "cohesion", "discriminating", "load-bearing", "grounded". Say the plain thing instead.
- **Answer the question that was asked, then stop.** Do not append findings, caveats, or
  next-step menus the author did not ask for.
- Default to a few sentences. Reach for a table only when comparing three or more things on the
  same axes — not to decorate a list.
- Technical precision still wins over brevity for: exact error text, shell commands, code, wire
  formats, security warnings, and irreversible actions. Never blur those to sound simpler.

**Do not let a discovery hijack the request.** Finding a real bug mid-task is useful; turning the
reply into a report about it is not. State it in one or two sentences, ask whether to act on it,
and return to what was actually asked. Do not investigate it first — ask, then investigate if told
to.

**One topic per reply.** Never stack findings. If two things turn up, raise the one that blocks the
current request and hold the other until asked. A reply that opens three threads at once forces the
author to triage work they did not ask for, and it reads as the project falling apart when it is
not.

**Never dump a full backlog unprompted.** Lists of every known gap, bug, and risk are for when the
author asks "what is left". Volunteering them mid-task buries the answer and destroys the sense of
where the project actually stands.

## Repository status

The Cargo workspace exists (root `Cargo.toml`, `resolver = "3"`, Edition 2024). It has **three members**: `crates/clauders` (a Claude SDK crate), `crates/openrouter-rs` (an OpenRouter SDK crate), and `crates/airs-transport` (a generic async transport substrate with an HTTP/reqwest layer, shared by the two SDK crates). Add new members under `crates/` only when there is concrete work for them; do not pre-create speculative crates.

## Project intent

`airsstack` is the author's personal AI technology stack, written in Rust. The `clauders` crate's
driving objective is **100% feature parity and behavioral compatibility with Anthropic's official
SDKs** across three pillars — the **Messages API** (base SDK), the **Agent SDK** (drives the `claude`
Code CLI as a subprocess), and **Managed Agents** (server-hosted stateful agents). A Rust caller gets
the same capabilities a Python or TypeScript caller gets, with idiomatic Rust ergonomics. The
authoritative statement is [`crates/clauders/docs/vision-and-strategy.md`](crates/clauders/docs/vision-and-strategy.md).

The earlier token-efficiency / mixed-routing thesis (route sub-tasks to cheaper non-Claude models via
OpenRouter) is **shelved, not abandoned** — it returns only under the vision doc's §8 re-introduction
criteria, once all three pillars are at parity. Do not design for it now.

Inspirations called out in the README (LangChain, CrewAI, DSPy, DeepEval, BeeAI; Claude Code,
OpenCode, Gemini CLI) remain reference points for ergonomics, not a mandate to clone any one of them.

## Scope discipline

Be pragmatic; do not build for an imagined future. The repo ships only what serves the parity target.
Today that is the `clauders` crate (the three-pillar parity client) and the `openrouter-rs` crate (an
**independent** standalone OpenRouter SDK — the former `OpenRouterRuntime`/`RoutingRuntime` integration
into `clauders` was severed per vision §9.1; the crate itself is kept).

The native superset that predated the parity pivot — `ApiRuntime` (native Messages loop), cross-provider
routing, the middleware/evals/orchestration framework tier, and the native permission/judge/subagent/
session engines — has been **removed** (vision §5). None of it exists in the official SDKs, so none
belongs in a parity-first `clauders`. Do not reintroduce these or the obsolete crate names
(`airsstack-cli`, `airsstack-core`, `provider-claude`, `provider-openrouter`, `airsdsp`). If the author
decides to add a crate or re-introduce a removed subsystem, it is named and scoped at that point under
the vision doc's §8 criteria.

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
