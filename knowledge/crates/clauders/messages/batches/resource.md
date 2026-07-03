---
type: Rust Endpoint
title: clauders::messages::batches::resource::BatchesResource
description: HTTP dispatch for the Message Batches API — create, get, list, results, cancel, delete against /v1/messages/batches.
tags: [rust, sdk, anthropic, messages-api, batches, http, endpoint]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/batches/resource.rs
---

`BatchesResource<'a, T: HttpTransport>` is obtained via
`client.messages().batches()` (see
[MessagesResource::batches](/crates/clauders/messages/resource.md)); never
constructed directly. Path prefix: `v1/messages/batches`.

# Schema

- `create(&self, req: BatchRequest) -> Result<Batch, Error>` — `POST /v1/messages/batches`.
  The batch transitions `in_progress` → `ended` asynchronously; poll `get`.
- `get`, `list`, `results` (returns
  [BatchResultStream](/crates/clauders/messages/batches/results.md)),
  `cancel`, `delete` — round out the batch lifecycle.

All methods decode 2xx bodies into the appropriate
[batch types](/crates/clauders/messages/batches/types.md) and non-2xx bodies
into [Error](/crates/clauders/error.md) via shared `wire_helpers`.

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::error::Error> {
use clauders::Client;
use clauders::messages::{BatchRequest, MessageRequest};
use clauders::types::{ApiKey, CustomRequestId, MaxTokens, ModelId};
let client = Client::builder()?.api_key(ApiKey::new("sk-ant-…").unwrap()).build()?;
let batch_req = BatchRequest::builder()
    .add(CustomRequestId::new("r1").unwrap(),
         MessageRequest::builder().model(ModelId::claude_sonnet_4_5())
             .max_tokens(MaxTokens::new(16).unwrap()).add_user_text("hi").build())
    .build();
let batch = client.messages().batches().create(batch_req).await?;
println!("batch id: {}", batch.id);
# Ok(()) }
```

Related: [batch types](/crates/clauders/messages/batches/types.md),
[BatchResultStream](/crates/clauders/messages/batches/results.md),
[MessagesResource](/crates/clauders/messages/resource.md).

# Citations

1. `crates/clauders/src/messages/batches/resource.rs`
