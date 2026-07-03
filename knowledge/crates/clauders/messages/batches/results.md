---
type: Rust Module
title: clauders::messages::batches::results
description: BatchResultStream — an async JSONL line-splitting Stream over decoded BatchResultRow values from a batch results response body.
tags: [rust, sdk, anthropic, messages-api, batches, streaming, jsonl]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/batches/results.rs
---

Kept separate from HTTP dispatch ([resource.rs](/crates/clauders/messages/batches/resource.md))
to isolate the line-splitting/stream-driving logic.

# Schema

```rust
pub struct BatchResultStream {
    body: BodyStream,
    buf: BytesMut,
    terminated: bool,
}
```

Implements `Stream<Item = Result<BatchResultRow, Error>>`. Each item is one
JSONL line decoded from the body of
`GET /v1/messages/batches/{id}/results`. Yields `Error::JsonLines` when a
line cannot be decoded, `Error::Transport` when the underlying body stream
errors; the stream terminates after any error. `try_split_line` pulls the
next newline-terminated line out of the internal buffer without the
newline byte.

Related: [BatchesResource::results](/crates/clauders/messages/batches/resource.md),
[BatchResultRow](/crates/clauders/messages/batches/types.md),
[Error::JsonLines](/crates/clauders/error.md).

# Citations

1. `crates/clauders/src/messages/batches/results.rs`
