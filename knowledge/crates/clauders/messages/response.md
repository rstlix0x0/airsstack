---
type: Rust Module
title: clauders::messages::response
description: Decoded Messages API response types — Message, StopReason, and Usage (with prompt-caching token breakdown).
tags: [rust, sdk, anthropic, messages-api, response]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/response.rs
---

Kept apart from request construction ([request.rs](/crates/clauders/messages/request.md))
so the response envelope can evolve independently. HTTP transport and
envelope unwrapping live in [resource.rs](/crates/clauders/messages/resource.md).

# Schema

```rust
pub struct Message {
    pub id: MessageId,
    pub kind: MessageKind,       // always Message on the wire
    pub role: Role,              // always Assistant for responses
    pub model: ModelId,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<StopSequence>,
    pub usage: Usage,
}

pub enum StopReason {
    EndTurn, MaxTokens, StopSequence,
    ToolUse,   // feature messages-tools
    Refusal,   // feature messages-structured-outputs
}

pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: Option<u32>, // feature messages-caching
    pub cache_read_input_tokens: Option<u32>,     // feature messages-caching
    pub cache_creation: Option<CacheCreation>,    // feature messages-caching
}

pub struct CacheCreation {                        // feature messages-caching
    pub ephemeral_5m_input_tokens: u32,
    pub ephemeral_1h_input_tokens: u32,
}
```

`Usage::total_input_tokens()` (feature `messages-caching`) sums
`input_tokens + cache_creation_input_tokens + cache_read_input_tokens`,
saturating on overflow.

# Examples

```rust
use clauders::messages::response::{Message, StopReason};
let j = r#"{"id":"msg_01","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"Hi"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":25,"output_tokens":5}}"#;
let msg: Message = serde_json::from_str(j).unwrap();
assert_eq!(msg.stop_reason, Some(StopReason::EndTurn));
```

Related: [MessagesResource::create](/crates/clauders/messages/resource.md),
[MessageId / StopSequence](/crates/clauders/types/ids.md),
[CacheControl](/crates/clauders/types/caching.md),
[streaming StreamEvent](/crates/clauders/messages/streaming.md) (`MessageStart`
carries an initial `Message` shell).

# Citations

1. `crates/clauders/src/messages/response.rs`
