---
type: Rust Module
title: clauders::agent::message
description: Message — the exhaustive, internally-tagged enum of top-level frames streamed from the claude binary's stdout (Assistant, User, System, Result, StreamEvent).
tags: [rust, sdk, agent, message-frames]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/message.rs
---

# Schema

```rust
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Assistant(AssistantMessage),
    User(UserMessage),
    System(SystemMessage),
    Result(ResultMessage),
    StreamEvent(StreamEvent),
}

pub struct AssistantMessage {
    pub content: Vec<ContentBlock>, // lifted from wire's nested {"message":{"content":[...]}} via content_from_message
    pub parent_tool_use_id: Option<String>,
}

pub struct UserMessage { pub message: serde_json::Value, pub parent_tool_use_id: Option<String> }
pub struct SystemMessage { pub subtype: Option<String>, pub extra: serde_json::Value } // #[serde(flatten)]

pub struct ResultMessage {
    pub result: String,
    pub is_error: bool,
    pub total_cost_usd: Option<f64>,
    pub stop_reason: Option<String>,
    pub usage: Option<Usage>,
    pub session_id: SessionId,
    pub num_turns: u32,
}

pub struct StreamEvent { pub event: serde_json::Value } // opaque, shape varies by event

pub struct Usage { pub input_tokens: u64, pub output_tokens: u64 } // agent-local, tolerant subset
```

`AssistantMessage`'s `Deserialize` and `Serialize` impls are NOT inverses:
deserializing lifts `content` out of the wire's nested `message` object, but
serializing would emit `content` as a bare array — do not rely on a
round-trip for this type.

# Examples

```rust
use clauders::agent::Message;
let json = r#"{"type":"result","subtype":"success","result":"done","is_error":false,"session_id":"s1","num_turns":3}"#;
let msg: Message = serde_json::from_str(json).unwrap();
assert!(matches!(msg, Message::Result(_)));
```

Related: [ContentBlock](/crates/clauders/agent/content.md),
[SessionId](/crates/clauders/agent/types/session-id.md),
[MessageStream](/crates/clauders/agent/stream.md) (yields `Result<Message, AgentError>`),
[Demux::route](/crates/clauders/agent/cli/demux.md) (routes `Message` frames
to the active turn sink and clears it on `Result`).

# Citations

1. `crates/clauders/src/agent/message.rs`
