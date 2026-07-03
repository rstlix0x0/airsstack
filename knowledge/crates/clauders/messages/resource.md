---
type: Rust Endpoint
title: clauders::messages::resource::MessagesResource
description: The Messages API HTTP dispatch handle — create (POST /v1/messages), stream (SSE), count_tokens, and batches() — borrowed from a Client.
tags: [rust, sdk, anthropic, messages-api, http, endpoint]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/resource.rs
---

`MessagesResource<'a, T: HttpTransport>` is a short-lived handle obtained via
`client.messages()` on [Client](/crates/clauders/client.md); it is never
constructed directly. Retry logic lives at the client layer; this module
owns request assembly, dispatch, and response decoding only.

# Schema

- `create(&self, req: MessageRequest) -> Result<Message, Error>` —
  `POST v1/messages`, serializes the body, sets `content-type`,
  `anthropic-version`, `x-api-key`, and `anthropic-beta` headers, and
  decodes a 2xx body as [Message](/crates/clauders/messages/response.md)
  or a non-2xx body as an [Error](/crates/clauders/error.md).
- `stream(&self, req: MessageRequest) -> Result<streaming::MessageStream, Error>`
  (feature `messages-streaming`) — sets `stream = true` and
  `Accept: text/event-stream`; checks the HTTP status eagerly before
  yielding the stream (a non-2xx response is decoded as an error
  immediately, never wrapped in the stream).
- `count_tokens(&self, req: MessageRequest) -> Result<token_counting::TokenCount, Error>`
  (feature `messages-token-counting`) — serializes a `CountTokensBody`
  projection that omits fields the endpoint rejects (`max_tokens`,
  `temperature`, `top_p`, `top_k`, `stop_sequences`, `metadata`, `stream`).
- `batches(&self) -> batches::resource::BatchesResource<'_, T>`
  (feature `messages-batches`).

Path constant: `v1/messages` (no leading slash — relies on `BaseUrl::join`
segment-resolution semantics).

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::error::Error> {
use clauders::Client;
use clauders::messages::MessageRequest;
use clauders::types::{ApiKey, MaxTokens, ModelId};
let client = Client::builder()?.api_key(ApiKey::new("sk-ant-…").unwrap()).build()?;
let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(1024).unwrap())
    .add_user_text("Hello!")
    .build();
let msg = client.messages().create(req).await?;
# Ok(()) }
```

Related: [MessageRequest](/crates/clauders/messages/request.md),
[Message response](/crates/clauders/messages/response.md),
[MessageStream](/crates/clauders/messages/streaming.md),
[TokenCount](/crates/clauders/messages/token-counting.md),
[BatchesResource](/crates/clauders/messages/batches/resource.md),
[Error hierarchy](/crates/clauders/error.md).

# Citations

1. `crates/clauders/src/messages/resource.rs`
