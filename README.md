# airsstack

The original motivation behind this project actually is because there are so many AI application's solution today which scattered. I've been built this project to provides my own personal AI technology stacks.

This project inspired from multiple solutions today:

- [LangChain](https://www.langchain.com/)
- [CrewAI](https://crewai.com/)
- [DSPy](https://dspy.ai/)
- [DeepEval - The Open-Source LLM Evaluation Framework](https://deepeval.com/)
- [Welcome to the BeeAI Framework - BeeAI Framework](https://framework.beeai.dev/introduction/welcome)

CLI agent

- [Claude Code \| Claude](https://www.claude.com/product/claude-code)
- [OpenCode \| The AI coding agent built for the terminal](https://opencode.ai/)
- [Build, debug & deploy with AI \| Gemini CLI](https://geminicli.com/)

I decided to standardize on `Claude` as the advanced model tier. Rather than build a broad bespoke
framework, `clauders` — the Claude SDK crate — now targets **100% feature parity and behavioral
compatibility with Anthropic's official SDKs**, so a Rust caller gets the same capabilities a Python
or TypeScript caller gets, with idiomatic Rust ergonomics. It covers three official surfaces: the
**Messages API** (base SDK), the **Agent SDK** (drives the `claude` Code CLI), and **Managed Agents**
(server-hosted stateful agents). The Messages API and Agent SDK are implemented; Managed Agents is
not started. The pillar map and the crate's internal structure are in
[`crates/clauders/docs/architecture.md`](crates/clauders/docs/architecture.md), and the docs index is
[`crates/clauders/docs/README.md`](crates/clauders/docs/README.md).

Token-efficiency via mixed routing to cheaper models remains a longer-term direction, deliberately
shelved until the parity core is complete. `openrouter-rs` stays in the workspace as an independent
standalone SDK crate, no longer wired into `clauders`.

## Rust crates (the AI-SDK & Tools)

A Cargo workspace (`crates/`) with three members:

- **`clauders`** — a Claude SDK crate (Messages API, batches, structured outputs, streaming, tool use, prompt caching).
- **`openrouter-rs`** — an OpenRouter SDK crate (chat, streaming, tool calling, structured outputs, provider routing, dual caching, model catalog).
- **`airs-transport`** — a generic async transport substrate with an HTTP/reqwest layer, shared by the two SDK crates above.

> `clauders` and `openrouter-rs` are independent crates. The earlier plan to route between them
> lives in the vision doc's §8 re-introduction criteria, not in current scope.

Standard Rust commands apply: `cargo build`, `cargo test -p <crate>`, `cargo clippy`, `cargo fmt`.
The full pass/fail gate is a [cargo-make](https://github.com/sagiegurari/cargo-make) task —
`cargo make dod` (or `cargo make dod-crate <crate>` while working in one crate) — defined in
`Makefile.toml` and run unchanged by CI.

## The airsstack Claude Code plugin suite (the methodology)

A marketplace (`.claude-plugin/marketplace.json`) of seven plugins under `plugins/` that package this project's spec-driven, review-gated development methodology for [Claude Code](https://www.claude.com/product/claude-code):

| Plugin | What it provides |
| --- | --- |
| **`airsstack`** | Execution engine: a TDD `coder`, a merged code+spec `reviewer`, a claim `verifier`, a read-only `explorer`, an `orchestrate` driver, process guidelines, project-local memory, and a `concise` output mode (inspired by the [caveman](https://github.com/juliusbrussee/caveman) plugin). |
| **`airsstack-sdd`** | Spec-driven workflow: `brainstorm` an idea into a spec → `write-plan` (one objective per plan) → `execute-plan` with review checkpoints. Adapted from the [superpowers](https://github.com/obra/superpowers) plugin with airsstack-specific adjustments. |
| **`airsstack-guideline-rust`** | Rust engineering guidelines + a strict Definition-of-Done, delivered as a lazily-loaded skill the execution agents consult when touching Rust. |
| **`airsstack-journal`** | Transparent, note-based experiential memory: an Obsidian-compatible journal vault with a deterministic, embedding-free recall index (`capture` / `note` / `recall` / `review`). |
| **`airsstack-plugin-dev`** | Plugin-development toolkit — the workshop the rest of the suite is built in. v1 `cache-sync` installs a `PostToolUse` hook that mirrors in-tree `plugins/<plugin>/` edits into the per-version install cache, so a `SKILL.md` body edit goes live mid-session without a reinstall. |
| **`airsstack-cmux`** | Native [cmux](https://cmux.com) terminal control as four lazily-loaded skills (`cmux-control` hub, `cmux-workspace`, `cmux-browser`, `cmux-config`) over the real `cmux` CLI plus helper scripts. Requires a cmux install on the machine. |
| **`airsstack-okf`** | Open Knowledge Format (OKF) v0.1 producer+consumer toolkit: provision a repo-local knowledge bundle, author/batch-enrich concept documents, recall with progressive disclosure, and lint conformance deterministically. |

The plugins are language-agnostic except for the guideline plugin: the agents obtain their Definition-of-Done and rules from whichever `*-guideline-*` skill is installed and degrade gracefully when none is present. Upstream attribution for `airsstack-sdd` (superpowers) and `airsstack` (caveman) lives in each plugin's own README.

### Using the plugin suite

Working inside this repository, the suite loads automatically — `.claude/settings.json` registers the in-repo marketplace and enables all seven plugins (restart Claude Code once to activate).

To use it in another project, install from the GitHub marketplace:

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack@airsstack
/plugin install airsstack-sdd@airsstack
/plugin install airsstack-guideline-rust@airsstack
/plugin install airsstack-journal@airsstack
/plugin install airsstack-plugin-dev@airsstack
/plugin install airsstack-cmux@airsstack
/plugin install airsstack-okf@airsstack
```

Each plugin has its own README under `plugins/<name>/` with the full component list. Everything is namespaced (`airsstack:<name>`, `airsstack-sdd:<name>`, `airsstack-journal:<name>`, …).

## License

Apache-2.0. See [LICENSE](./LICENSE).
