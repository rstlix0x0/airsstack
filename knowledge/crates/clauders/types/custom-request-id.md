---
type: Rust Newtype
title: clauders::types::CustomRequestId
description: Caller-supplied identifier correlating a batch row with its result, non-empty-validated and distinct from BatchId at the type level.
tags: [rust, sdk, newtype, batches, identifier]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/custom_request_id.rs
---

Feature-gated behind `messages-batches`. Supplied by the caller when
building a [BatchRequest](/crates/clauders/messages/batches/types.md);
returned unchanged in each `BatchResultRow` so callers can map results back
to their original inputs.

# Schema

```rust
pub struct CustomRequestId(String); // #[serde(transparent)]
pub struct InvalidCustomRequestId;  // "CustomRequestId must not be empty"
```

`as_str()`, `Display`.

# Examples

```rust
use clauders::types::CustomRequestId;
let id = CustomRequestId::new("my-row-1").unwrap();
assert_eq!(id.as_str(), "my-row-1");
assert!(CustomRequestId::new("").is_err());
```

Related: [BatchId](/crates/clauders/types/batch-id.md) (the
server-generated counterpart), [batch types](/crates/clauders/messages/batches/types.md).

# Citations

1. `crates/clauders/src/types/custom_request_id.rs`
