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

Previously, I've been doing lot of experiments with multiple models, and then I decide to only use `Claude` models as the only advanced models. But the next problem is, it's too expensive related with it's high token consumptions, especially if we are too tightly coupled with `Claude Code`. So I'm starting to thinking to provides my own AI-SDK & AI-Agent SDK for my personal usages, by still utilizing `Claude` but combined with other alternative models through `OpenRouter`, such as:

- `DeepSeek`
- `Kimi K2`
- `Qwen`

## Rust crates (the AI-SDK & Tools)

A Cargo workspace (`crates/`) with two members:

- **`clauders`** — a Claude SDK crate (Messages API, batches, structured outputs, streaming, tool use, prompt caching).
- **`openrouter-rs`** — an OpenRouter SDK crate (chat, streaming, tool calling, structured outputs, provider routing, dual caching, model catalog).

Standard Rust commands apply: `cargo build`, `cargo test -p <crate>`, `cargo clippy`, `cargo fmt`.

## The airsstack Claude Code plugin suite (the methodology)

A marketplace (`.claude-plugin/marketplace.json`) of three plugins under `plugins/` that package this project's spec-driven, review-gated development methodology for [Claude Code](https://www.claude.com/product/claude-code):

| Plugin | What it provides |
| --- | --- |
| **`airsstack`** | Execution engine: a TDD `coder`, a merged code+spec `reviewer`, a claim `verifier`, a read-only `explorer`, an `orchestrate` driver, process guidelines, project-local memory, and a `concise` output mode. |
| **`airsstack-sdd`** | Spec-driven workflow: `brainstorm` an idea into a spec → `write-plan` (one objective per plan) → `execute-plan` with review checkpoints. |
| **`airsstack-guideline-rust`** | Rust engineering guidelines + a strict Definition-of-Done, delivered as a lazily-loaded skill the execution agents consult when touching Rust. |

The plugins are language-agnostic except for the guideline plugin: the agents obtain their Definition-of-Done and rules from whichever `*-guideline-*` skill is installed and degrade gracefully when none is present.

## Using the plugin suite

Working inside this repository, the suite loads automatically — `.claude/settings.json` registers the in-repo marketplace and enables all three plugins (restart Claude Code once to activate).

To use it in another project, install from the GitHub marketplace:

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack@airsstack
/plugin install airsstack-sdd@airsstack
/plugin install airsstack-guideline-rust@airsstack
```

Each plugin has its own README under `plugins/<name>/` with the full component list. Everything is namespaced (`airsstack:<name>`, `airsstack-sdd:<name>`, …).

## License

Apache-2.0. See [LICENSE](./LICENSE).
