---
type: Rust Endpoint
title: clauders::models::resource::ModelsResource
description: HTTP dispatch handle for GET /v1/models and GET /v1/models/{id} — lists and fetches Claude model metadata.
tags: [rust, sdk, anthropic, models-api, http, endpoint]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/models/resource.rs
---

Feature-gated (`models`, not enabled by default). `ModelsResource<'a, T: HttpTransport>`
is obtained via `client.models()` on [Client](/crates/clauders/client.md);
never constructed directly.

# Schema

- `list(&self) -> Result<ModelList, Error>` — `GET v1/models`; returns the current page of results.
- `get(&self, id: &ModelId) -> Result<ModelInfo, Error>` — `GET v1/models/{id}`.

Both decode 2xx bodies into [models types](/crates/clauders/models/types.md)
and non-2xx bodies into [Error](/crates/clauders/error.md).

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::error::Error> {
use clauders::Client;
use clauders::types::ApiKey;
let client = Client::builder()?.api_key(ApiKey::new("sk-ant-…").unwrap()).build()?;
let list = client.models().list().await?;
println!("{} models available", list.data.len());
# Ok(()) }
```

Related: [ModelInfo / ModelList](/crates/clauders/models/types.md),
[ModelId](/crates/clauders/types/model-id.md),
[Client](/crates/clauders/client.md).

# Citations

1. `crates/clauders/src/models/resource.rs`
2. `crates/clauders/src/models/mod.rs`
