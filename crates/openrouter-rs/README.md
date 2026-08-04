# openrouter-rs

Unofficial Rust SDK for the [OpenRouter API](https://openrouter.ai/docs).
OpenRouter is a unified, OpenAI-compatible gateway that routes chat-completion
requests across many model providers behind a single API key.
Not affiliated with OpenRouter.

## Status

v0.1.0 — chat completions (non-streaming and SSE streaming), tool calling,
structured outputs, provider routing preferences, dual caching (prompt cache +
edge cache), and the model catalog.

Two endpoints are implemented: `POST /chat/completions` and `GET /models`.

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

## Documentation

Full documentation lives in [`docs/`](docs/), organised by
[Diátaxis](https://diataxis.fr/):

- **[Tutorials](docs/README.md#tutorials)** — [your first
  completion](docs/tutorials/01-first-completion.md), then [streaming a
  response](docs/tutorials/02-streaming-responses.md).
- **[How-to guides](docs/README.md#how-to-guides)** — [call
  tools](docs/how-to/call-tools.md), [request structured
  outputs](docs/how-to/request-structured-outputs.md), [steer provider
  routing](docs/how-to/steer-provider-routing.md), [cache
  requests](docs/how-to/cache-requests.md), [handle
  errors](docs/how-to/handle-errors.md), [test with a mock
  transport](docs/how-to/test-with-a-mock-transport.md).
- **[Reference](docs/README.md#reference)** — per-module tables for requests,
  responses, streaming, caching, routing, the catalog, the domain newtypes, and
  the error surface.
- **[Explanation](docs/README.md#explanation)** —
  [architecture](docs/explanation/architecture.md), [type-state
  builders](docs/explanation/type-state-builders.md), [validated domain
  types](docs/explanation/validated-domain-types.md), [the two
  caches](docs/explanation/the-two-caches.md), [errors and
  retries](docs/explanation/errors-and-retries.md).

API docs: `cargo doc -p openrouter-rs --no-deps --open`.

## Design in one screen

- **Featureless.** No Cargo `[features]`; every capability is always compiled.
  `--all-features` equals the default build.
- **Type-state builders.** A missing `api_key`, `model`, or `messages` is a
  compile error, not a runtime one — proven by `trybuild` fixtures under
  `tests/compile_fail/`.
- **Parse, don't validate.** Validated newtypes (`ApiKey`, `ModelId`,
  `Temperature`, `StopSequences`, …) carry their invariants, so request building
  is infallible.
- **Static dispatch.** The transport is a generic parameter,
  `Client<T: HttpTransport>`, never a trait object. `DefaultClient` is
  `Client<ReqwestTransport>`.
- **No foreign error types on the public surface.** `reqwest` failures become
  `TransportError`; `url::Url` stays private inside `BaseUrl`.
- **No retry layer.** `Error::is_retryable()` and `Error::retry_after()` give you
  the signal; the policy is yours.

## Examples

Each hits the live API and reads the key from the environment:

```text
OPENROUTER_API_KEY=sk-... cargo run --example 01_chat
OPENROUTER_API_KEY=sk-... cargo run --example 02_streaming
OPENROUTER_API_KEY=sk-... cargo run --example 03_tools
OPENROUTER_API_KEY=sk-... cargo run --example 04_caching
```

## License

Apache-2.0. See the workspace root `LICENSE` file.
