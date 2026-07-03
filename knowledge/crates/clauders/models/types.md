---
type: Rust Module
title: clauders::models::types
description: Response types for the Models resource — ModelInfo per-model record, ModelInfoKind discriminant, and the paginated ModelList wrapper.
tags: [rust, sdk, anthropic, models-api, wire-types]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/models/types.rs
---

Decoupled from HTTP dispatch in
[resource.rs](/crates/clauders/models/resource.md).

# Schema

```rust
pub enum ModelInfoKind { Model }  // currently the only variant

pub struct ModelInfo {
    pub id: ModelId,
    pub display_name: String,
    pub created_at: String,   // kept as String for format forward-compat
    pub kind: ModelInfoKind,  // wire field "type"
}

pub struct ModelList {
    pub data: Vec<ModelInfo>,
    pub has_more: bool,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
}
```

# Examples

```rust
use clauders::models::ModelInfo;
let json = r#"{"id":"claude-sonnet-4-5","display_name":"Claude Sonnet 4.5","created_at":"2025-09-01T00:00:00Z","type":"model"}"#;
let info: ModelInfo = serde_json::from_str(json).unwrap();
assert_eq!(info.display_name, "Claude Sonnet 4.5");
```

Related: [ModelsResource](/crates/clauders/models/resource.md),
[ModelId](/crates/clauders/types/model-id.md).

# Citations

1. `crates/clauders/src/models/types.rs`
