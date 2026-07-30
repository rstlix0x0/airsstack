# clauders

Unofficial Rust SDK for the [Anthropic Messages API](https://docs.anthropic.com/en/api/messages).
Not affiliated with Anthropic.

## Status

v0.1.0 — the full Messages API surface is implemented, including streaming,
tool use, prompt caching, token counting, message batches, and structured outputs.

## Quick start

```rust,no_run
use clauders::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()?
        .api_key(ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?)
        .build()?;

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(1024))
        .add_user_text("Say hi.")
        .build();

    let msg = client.messages().create(req).await?;
    println!("{:?}", msg.stop_reason);
    Ok(())
}
```

## Capabilities

The crate carries no Cargo features; every capability below is always compiled.

| Capability | What it provides |
|---|---|
| Messages | `POST /v1/messages` request/response types and `MessagesResource` |
| Streaming | SSE streaming via `MessageStream` |
| Tools | Tool (function calling) types |
| Caching | Prompt-caching fields and cache-hit counters on `Usage` |
| Transport | Default HTTP transport backed by `reqwest` with `rustls` |
| Token counting | `POST /v1/messages/count_tokens` helper |
| Batches | Message Batches API (`/v1/messages/batches`) |
| Structured outputs | Constrain responses to a JSON Schema via `OutputConfig` |
| Models | `GET /v1/models` and `GET /v1/models/{id}` |

## Examples

Two ladders, each simplest-first, with a `README.md` per example:

- [`examples/messages/`](examples/messages/README.md) — the Messages API. Needs
  `ANTHROPIC_API_KEY`.
- [`examples/agent/`](examples/agent/README.md) — the Agent SDK. Needs a `claude`
  binary (2.0.0+) on `PATH`; no API key, since the SDK drives that binary rather
  than calling the API itself.

```text
ANTHROPIC_API_KEY=sk-... cargo run --example 01_hello
ANTHROPIC_API_KEY=sk-... cargo run --example 02_streaming

cargo run --example agent_01_query
cargo run --example agent_14_agent_console
```

## License

Apache-2.0. See the workspace root `LICENSE` file.
