---
type: Rust Module
title: clauders::prelude
description: Pure re-export module grouping the imports most call sites need for a single `use clauders::prelude::*;`.
tags: [rust, sdk, prelude]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/prelude.rs
---

No logic lives here; it is an export-only module (exempt from the crate's
unit-test mandate for that reason).

# Schema

Always re-exported: `AnthropicVersion`, `ApiKey`, `BetaHeader`, `MaxTokens`,
`ModelId`, `Temperature`, `TopK`, `TopP`, `ApiError`, `BuildError`, `Client`,
`Error`, `TransportError`.

Feature-gated re-exports:
- `messages`: `ContentBlock`, `Message`, `MessageRequest`, `Role`, `StopReason`.
- `messages-streaming`: `MessageStream`, `StreamEvent`.

# Examples

```rust,no_run
use clauders::prelude::*;
let client = Client::builder()?
    .api_key(ApiKey::new("sk-ant-...").unwrap())
    .build()?;
# Ok::<(), clauders::Error>(())
```

Related: [Client](/crates/clauders/client.md), [messages overview](/crates/clauders/messages/overview.md).

# Citations

1. `crates/clauders/src/prelude.rs`
