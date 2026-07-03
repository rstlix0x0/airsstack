---
type: Rust Newtype
title: clauders::types::BatchId
description: Opaque server-generated identifier for a message batch, non-empty-validated so it cannot be swapped for another identifier type at compile time.
tags: [rust, sdk, newtype, batches, identifier]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/batch_id.rs
---

Feature-gated behind `messages-batches`. `BatchId::new(s) -> Result<Self, InvalidBatchId>`
preserves the string verbatim beyond a non-empty check.

# Schema

```rust
pub struct BatchId(String); // #[serde(transparent)]
pub struct InvalidBatchId;  // "BatchId must not be empty"
```

`as_str()`, `Display`.

# Examples

```rust
use clauders::types::BatchId;
let id = BatchId::new("msgbatch_01").unwrap();
assert_eq!(id.as_str(), "msgbatch_01");
assert!(BatchId::new("").is_err());
```

Related: [Batch (status object)](/crates/clauders/messages/batches/types.md),
[BatchesResource::get](/crates/clauders/messages/batches/resource.md),
[CustomRequestId](/crates/clauders/types/custom-request-id.md) (the
caller-supplied counterpart), [ID newtype family](/crates/clauders/types/ids.md).

# Citations

1. `crates/clauders/src/types/batch_id.rs`
