---
type: Rust Module
title: clauders::messages::batches::types
description: Wire types for the Message Batches API — BatchRequest input, Batch/BatchStatus status objects, and BatchResult/DeletedMessageBatch outputs.
tags: [rust, sdk, anthropic, messages-api, batches, wire-types]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/batches/types.rs
---

Isolated from the core Messages API request/response types. HTTP dispatch
lives in [resource.rs](/crates/clauders/messages/batches/resource.md); JSONL
result streaming lives in [results.rs](/crates/clauders/messages/batches/results.md).

# Schema

- **Input**: `BatchRequest { requests: Vec<BatchedMessageRequest> }`, built via
  `BatchRequest::builder().add(custom_id, params).build()`;
  `BatchedMessageRequest { custom_id: CustomRequestId, params: MessageRequest }`.
- **Status**: `Batch` (id, `BatchKind`, `BatchStatus`, `RequestCounts`, …),
  `BatchList` (paginated), `RequestCounts`.
- **Result**: `BatchResultRow`, `BatchResult`, `DeletedMessageBatch`,
  `DeletedBatchKind`.

# Examples

```rust
use clauders::messages::{BatchRequest, MessageRequest};
use clauders::types::{CustomRequestId, MaxTokens, ModelId};
let req = BatchRequest::builder()
    .add(
        CustomRequestId::new("r1").unwrap(),
        MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(16).unwrap())
            .add_user_text("hello")
            .build(),
    )
    .build();
assert_eq!(req.requests.len(), 1);
```

Related: [BatchesResource](/crates/clauders/messages/batches/resource.md),
[BatchResultStream](/crates/clauders/messages/batches/results.md),
[BatchId / CustomRequestId](/crates/clauders/types/batch-id.md),
[MessageRequest](/crates/clauders/messages/request.md).

# Citations

1. `crates/clauders/src/messages/batches/types.rs`
