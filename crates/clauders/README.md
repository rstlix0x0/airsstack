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
        .max_tokens(MaxTokens::new(1024).unwrap())
        .add_user_text("Say hi.")
        .build();

    let msg = client.messages().create(req).await?;
    println!("{:?}", msg.stop_reason);
    Ok(())
}
```

## Features

Default features (no opt-in required):

| Feature | What it enables |
|---|---|
| `messages` | `POST /v1/messages` request/response types and `MessagesResource` |
| `messages-streaming` | SSE streaming via `MessageStream` |
| `messages-tools` | Tool (function calling) types |
| `messages-caching` | Prompt-caching fields and cache-hit counters on `Usage` |
| `transport-reqwest` | Default HTTP transport backed by `reqwest` with `rustls` |

Optional features (add to `Cargo.toml` features list):

| Feature | What it enables |
|---|---|
| `messages-token-counting` | `POST /v1/messages/count_tokens` helper |
| `messages-batches` | Message Batches API (`/v1/messages/batches`) |
| `messages-structured-outputs` | Constrain responses to a JSON Schema via `OutputConfig` |
| `models` | `GET /v1/models` and `GET /v1/models/{id}` |

## Examples

```text
ANTHROPIC_API_KEY=sk-... cargo run --example 01_hello
ANTHROPIC_API_KEY=sk-... cargo run --example 02_streaming
ANTHROPIC_API_KEY=sk-... cargo run --example 03_tools
ANTHROPIC_API_KEY=sk-... cargo run --example 04_caching
```

## License

Apache-2.0. See the workspace root `LICENSE` file.
