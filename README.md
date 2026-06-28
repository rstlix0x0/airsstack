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

A marketplace (`.claude-plugin/marketplace.json`) of six plugins under `plugins/` that package this project's spec-driven, review-gated development methodology for [Claude Code](https://www.claude.com/product/claude-code):

| Plugin | What it provides |
| --- | --- |
| **`airsstack`** | Execution engine: a TDD `coder`, a merged code+spec `reviewer`, a claim `verifier`, a read-only `explorer`, an `orchestrate` driver, process guidelines, project-local memory, and a `concise` output mode (inspired by the [caveman](https://github.com/juliusbrussee/caveman) plugin). |
| **`airsstack-sdd`** | Spec-driven workflow: `brainstorm` an idea into a spec → `write-plan` (one objective per plan) → `execute-plan` with review checkpoints. Adapted from the [superpowers](https://github.com/obra/superpowers) plugin with airsstack-specific adjustments. |
| **`airsstack-guideline-rust`** | Rust engineering guidelines + a strict Definition-of-Done, delivered as a lazily-loaded skill the execution agents consult when touching Rust. |
| **`airsstack-journal`** | Transparent, note-based experiential memory: an Obsidian-compatible journal vault with a deterministic, embedding-free recall index (`capture` / `note` / `recall` / `review`). |
| **`airsstack-plugin-dev`** | Plugin-development toolkit — the workshop the rest of the suite is built in. v1 `cache-sync` installs a `PostToolUse` hook that mirrors in-tree `plugins/<plugin>/` edits into the per-version install cache, so a `SKILL.md` body edit goes live mid-session without a reinstall. |
| **`airsstack-cmux`** | Native [cmux](https://cmux.com) terminal control as four lazily-loaded skills (`cmux-control` hub, `cmux-workspace`, `cmux-browser`, `cmux-config`) over the real `cmux` CLI plus helper scripts. Requires a cmux install on the machine. |

The plugins are language-agnostic except for the guideline plugin: the agents obtain their Definition-of-Done and rules from whichever `*-guideline-*` skill is installed and degrade gracefully when none is present. Upstream attribution for `airsstack-sdd` (superpowers) and `airsstack` (caveman) lives in each plugin's own README.

### Using the plugin suite

Working inside this repository, the suite loads automatically — `.claude/settings.json` registers the in-repo marketplace and enables all six plugins (restart Claude Code once to activate).

To use it in another project, install from the GitHub marketplace:

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack@airsstack
/plugin install airsstack-sdd@airsstack
/plugin install airsstack-guideline-rust@airsstack
/plugin install airsstack-journal@airsstack
/plugin install airsstack-plugin-dev@airsstack
/plugin install airsstack-cmux@airsstack
```

Each plugin has its own README under `plugins/<name>/` with the full component list. Everything is namespaced (`airsstack:<name>`, `airsstack-sdd:<name>`, `airsstack-journal:<name>`, …).

## License

Apache-2.0. See [LICENSE](./LICENSE).
