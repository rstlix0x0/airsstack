---
type: Rust Newtype
title: clauders::types::ModelId
description: Claude model identifier newtype — ModelId::custom accepts any current or future non-whitespace identifier; claude_* constructors are a frozen-at-build-time convenience.
tags: [rust, sdk, newtype, model, identifier]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/model_id.rs
---

`ModelId::custom` is the primary entry point — never goes stale. The
headline `claude_*` constructors (e.g. `claude_sonnet_4_5()`) are a
typo-proof, IDE-discoverable snapshot of models known at this SDK release;
the authoritative current list is the `models` resource or the upstream
`GET /v1/models` endpoint.

# Schema

```rust
pub struct ModelId(String); // #[serde(transparent)]

pub enum InvalidModelId {
    Empty,
    Whitespace, // ASCII or Unicode whitespace rejected
}
```

`ModelId::custom(s) -> Result<Self, InvalidModelId>`, `as_str()`, plus
headline convenience constructors like `claude_sonnet_4_5()`.

# Examples

```rust
use clauders::types::ModelId;
assert_eq!(ModelId::claude_sonnet_4_5().as_str(), "claude-sonnet-4-5");
let custom = ModelId::custom("claude-future-model-1").expect("valid id");
assert_eq!(custom.as_str(), "claude-future-model-1");
```

Related: [MessageRequest::model](/crates/clauders/messages/request.md),
[ModelInfo::id](/crates/clauders/models/types.md),
[ModelsResource::get](/crates/clauders/models/resource.md),
[agent Options::model](/crates/clauders/agent/options.md).

# Citations

1. `crates/clauders/src/types/model_id.rs`
