# Messages API examples

Runnable examples for the `clauders` Messages API — the typed Rust client over
Anthropic's `POST /v1/messages` and its companion endpoints. Each example lives
in its own directory with a `main.rs` and a `README.md` that walks through the
SDK calls it uses.

## Prerequisites

- A `clauders`-usable API key in `ANTHROPIC_API_KEY`.
- Model access on that key for the models the examples name (`claude-sonnet-4-5`,
  `claude-sonnet-5`). Swap the `ModelId::claude_*()` call if a model is not
  available to you.

## Run any example

Every example is registered by name in `crates/clauders/Cargo.toml`, so run it by
name from anywhere in the workspace:

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example <name>
```

The first run compiles the crate; reruns are fast.

## The examples

| # | Name | Shows |
|---|------|-------|
| 01 | `01_hello` | Minimal non-streaming request and response |
| 02 | `02_streaming` | Server-sent-events streaming of the reply |
| 03 | `03_tools` | Single tool (function-calling) round-trip |
| 04 | `04_caching` | Prompt caching across two calls |
| 05 | `05_structured_output` | Constrain the reply to a JSON Schema |
| 06 | `06_thinking` | Extended thinking with a token budget |
| 07 | `07_agentic_tool_loop` | Multi-turn loop that runs tools until done |
| 08 | `08_vision` | Image input (base64) |
| 09 | `09_document_citations` | Document input with citations |
| 10 | `10_batches` | Message Batches: submit, poll, stream results |
| 11 | `11_coding_agent` | Agentic coding CLI with a ratatui TUI: real file + `cargo` tools |

## The shape shared by every example

```rust
use clauders::prelude::*;

let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
let client = Client::builder()?.api_key(api_key).build()?;

let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())   // required
    .max_tokens(MaxTokens::new(1024))      // required
    .add_user_text("...")
    .build();

let msg = client.messages().create(req).await?;
```

`MessageRequest::builder()` is a type-state builder: `model` and `max_tokens` are
required, and the code does not compile until both are set. `client.messages()`
returns the Messages resource; `.create()` sends one request, `.stream()` streams
one, and `.batches()` opens the Batches sub-resource.
