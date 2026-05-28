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

I'm also need to build my own agentic tools for my personal usages, with the primary objective is to suppress token usages but still to maintain accuracy, reliability and maintainability output, especially for the software engineering.

## Sub Crates

`airsstack` will be developed using Rust, with currently I had several sub _crates_ in my minds:

- `airsstack/airstack-cli`
- `airsstack/airstack-core`
- `airsstack/provider-claude`
- `airsstack/provider-openrouter`
- `airsstack/airsdsp`

### airsstack/airsstack

For now, there will be only two possible crates:

- `airsstack-cli`
  - It's my personal CLI tool as a replacement of :
    - `Claude Code`
    - `OpenCode`
- `airsstack-core`
  - It's a core agentic framework
