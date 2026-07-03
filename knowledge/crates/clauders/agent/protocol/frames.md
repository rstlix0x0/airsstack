---
type: Rust Module
title: clauders::agent::protocol::frames
description: Serde types for the control-protocol wire frames — InboundFrame (untagged union of control_response / control_request / message), and the outbound control request/response shapes.
tags: [rust, sdk, agent, protocol, wire-types]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/protocol/frames.rs
---

# Schema

```rust
#[serde(untagged)] // control variants listed first so an explicit control_* type never mis-parses as a message
pub enum InboundFrame {
    ControlResponse(ControlResponse),
    ControlRequest(InboundControlRequest),
    Message(agent::message::Message),
}

pub struct ControlResponse { pub response: ControlResponseBody }

#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlResponseBody {
    Success { request_id: String, response: serde_json::Value },
    Error { request_id: String, error: String },
}
impl ControlResponseBody { pub fn request_id(&self) -> &str; }

pub struct InboundControlRequest { pub request_id: String, pub request: InboundRequestBody }

#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum InboundRequestBody {
    CanUseTool { tool_name: String, input: serde_json::Value, tool_use_id: Option<String>,
                 agent_id: Option<String>, blocked_path: Option<String>, decision_reason: Option<String>,
                 title: Option<String>, display_name: Option<String>, description: Option<String> },
    HookCallback { callback_id: String, input: serde_json::Value, tool_use_id: Option<String> },
}

pub struct OutboundControlRequest<'a> { pub kind: &'static str, pub request_id: &'a str, pub request: OutboundRequestBody }

#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum OutboundRequestBody { Interrupt, SetModel { model: String }, SetPermissionMode { mode: String }, McpStatus }

pub struct OutboundControlResponse { pub kind: &'static str, pub response: OutboundResponseBody }

#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum OutboundResponseBody {
    Success { request_id: String, response: serde_json::Value },
    Error { request_id: String, error: String },
}
```

Why `#[serde(untagged)]` on `InboundFrame`: a message frame's discriminant
lives in its own `type` field (`assistant`/`result`/…), which does not
collide with the `control_request`/`control_response` discriminants; serde
tries each variant in order and the first structural match wins.

# Examples

```rust
use clauders::agent::protocol::InboundFrame;
let line = r#"{"type":"result","subtype":"success","result":"ok","is_error":false,"session_id":"s1","num_turns":1}"#;
let frame: InboundFrame = serde_json::from_str(line).unwrap();
assert!(matches!(frame, InboundFrame::Message(_)));
```

Related: [codec::decode_inbound/encode_line](/crates/clauders/agent/protocol/codec.md),
[agent::message::Message](/crates/clauders/agent/message.md),
[Demux::route](/crates/clauders/agent/cli/demux.md),
[Dispatcher::handle](/crates/clauders/agent/cli/dispatch.md),
[PermissionContext](/crates/clauders/agent/permissions.md) (mirrors
`InboundRequestBody::CanUseTool`'s fields).

# Citations

1. `crates/clauders/src/agent/protocol/frames.rs`
