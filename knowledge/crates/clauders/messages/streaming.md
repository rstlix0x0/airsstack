---
type: Rust Module
title: clauders::messages::streaming
description: SSE streaming wrapper for the Messages API — StreamEvent union, ContentDelta/MessageMetaDelta/UsageDelta sub-types, and the MessageStream Stream adapter.
tags: [rust, sdk, anthropic, messages-api, streaming, sse]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/streaming.rs
---

Gated behind `messages-streaming` (depends on `messages`) so the
`eventsource-stream` dependency is only compiled when needed. Building the
HTTP request lives in [resource.rs](/crates/clauders/messages/resource.md)
(`MessagesResource::stream`); tool-use content deltas are not modelled.

# Schema

```rust
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart { message: Message },
    ContentBlockStart { index: u32, content_block: ContentBlock },
    ContentBlockDelta { index: u32, delta: ContentDelta },
    ContentBlockStop { index: u32 },
    MessageDelta { delta: MessageMetaDelta, usage: UsageDelta },
    MessageStop,
    Ping,
    Error { error: ApiErrorBody },
}

pub enum ContentDelta {
    TextDelta { text: String },
    ThinkingDelta { thinking: String /* + more variants */ },
}
```

`MessageStream` — a `Stream<Item = Result<StreamEvent, Error>>` wrapper
driving SSE parsing; enforces a terminal-on-error rule (once an `Error`
event or transport error is yielded, the stream ends).
`MessageStream::collect()` drains the stream and assembles the complete
[Message](/crates/clauders/messages/response.md) from its events.

# Examples

```rust
use clauders::messages::StreamEvent;
let json = r#"{"type":"message_stop"}"#;
let ev: StreamEvent = serde_json::from_str(json).unwrap();
assert!(matches!(ev, StreamEvent::MessageStop));
```

Related: [MessagesResource::stream](/crates/clauders/messages/resource.md),
[Message / Usage](/crates/clauders/messages/response.md),
[ContentBlock / TextBlock](/crates/clauders/messages/content.md),
[Error::Stream](/crates/clauders/error.md).

# Citations

1. `crates/clauders/src/messages/streaming.rs`
