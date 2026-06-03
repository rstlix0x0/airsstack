# openrouter-rs

Unofficial Rust SDK for the [OpenRouter API](https://openrouter.ai/docs).
OpenRouter is a unified, OpenAI-compatible gateway that routes chat-completion
requests across many model providers behind a single API key.
Not affiliated with OpenRouter.

## Status

v0.1.0 — chat completions (non-streaming and SSE streaming), tool calling,
structured outputs, provider routing preferences, dual caching (prompt cache +
edge cache), and the model catalog.

## Quick start

```rust,no_run
use openrouter_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    let req = ChatRequest::builder()
        .model(ModelId::custom("openai/gpt-4o-mini")?)
        .messages(vec![Message::user("Say hi in one word.")])
        .build();

    let completion = client.chat().send(req).await?;
    if let Some(choice) = completion.choices.first() {
        println!("{:?}", choice.message.content);
    }
    Ok(())
}
```

## Features

| Feature | Default | What it enables |
|---|:---:|---|
| `streaming` | ✓ | SSE streaming via `ChatResource::stream` / `ChatStream` |
| `transport-reqwest` | ✓ | Default HTTP transport backed by `reqwest` with `rustls` |
| `__test-mocks` | | Internal: exposes `MockHttpTransport` for downstream tests |

All API capabilities — tool calling, structured outputs, provider routing
preferences, caching, and the model catalog — are part of the core surface and
require no feature flag.

## Examples

```text
OPENROUTER_API_KEY=sk-... cargo run --example 01_chat
OPENROUTER_API_KEY=sk-... cargo run --example 02_streaming
OPENROUTER_API_KEY=sk-... cargo run --example 03_tools
OPENROUTER_API_KEY=sk-... cargo run --example 04_caching
```

## License

Apache-2.0. See the workspace root `LICENSE` file.
