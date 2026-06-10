//! Serde types for the control-protocol wire frames.
//!
//! Three inbound frame kinds arrive on the binary's stdout, discriminated by
//! the top-level `type` field:
//! - a **message** frame (`assistant`/`user`/`system`/`result`/`stream_event`)
//!   carrying model output, forwarded to the caller's message stream;
//! - a **`control_response`** replying to one of our outbound control requests,
//!   matched back to its waiter by `request_id`;
//! - an inbound **`control_request`** (`can_use_tool`/`hook_callback`) the
//!   binary issues to us mid-turn, answered with a correlated control response.
//!
//! Outbound, the runtime writes a user-message frame (the prompt) and
//! `control_request` frames (`interrupt`/`set_model`/…).

use serde::{Deserialize, Serialize};

use crate::agent::message::Message;

/// A frame read from the binary's stdout.
///
/// `untagged` is required because a message frame's discriminant lives in its
/// own `type` field (`assistant`/`result`/…), which does not collide with the
/// `control_request`/`control_response` discriminants; serde tries each
/// variant in order and the first structural match wins. Control variants are
/// listed first so an explicit `control_*` type is never mis-parsed as a
/// message.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum InboundFrame {
    /// A reply to one of our outbound control requests.
    ControlResponse(ControlResponse),
    /// A control request issued to us by the binary.
    ControlRequest(InboundControlRequest),
    /// A model-output message frame.
    Message(Message),
}

/// Wrapper for an inbound `control_response` frame.
#[derive(Debug, Deserialize)]
pub struct ControlResponse {
    /// The response body.
    pub response: ControlResponseBody,
}

/// Body of a `control_response`, tagged by `subtype`.
#[derive(Debug, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlResponseBody {
    /// The control request succeeded.
    Success {
        /// Correlation id matching our outbound request.
        request_id: String,
        /// Optional structured payload (e.g. mcp status).
        #[serde(default)]
        response: serde_json::Value,
    },
    /// The control request failed.
    Error {
        /// Correlation id matching our outbound request.
        request_id: String,
        /// Error detail.
        #[serde(default)]
        error: String,
    },
}

impl ControlResponseBody {
    /// The correlation id carried by either variant.
    #[must_use]
    pub fn request_id(&self) -> &str {
        match self {
            Self::Success { request_id, .. } | Self::Error { request_id, .. } => request_id,
        }
    }
}

/// An inbound `control_request` issued by the binary.
#[derive(Debug, Deserialize)]
pub struct InboundControlRequest {
    /// Server-minted id we must echo in our response.
    pub request_id: String,
    /// The request body.
    pub request: InboundRequestBody,
}

/// Body of an inbound control request, tagged by `subtype`.
#[derive(Debug, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum InboundRequestBody {
    /// The binary asks whether a tool may run.
    CanUseTool {
        /// Tool name.
        tool_name: String,
        /// Tool input (opaque).
        #[serde(default)]
        input: serde_json::Value,
    },
    /// The binary invokes a registered hook.
    HookCallback {
        /// Hook callback id / event name.
        #[serde(default)]
        callback_id: String,
        /// Hook input payload (opaque).
        #[serde(default)]
        input: serde_json::Value,
    },
}

/// An outbound `control_request` we send to the binary.
#[derive(Debug, Serialize)]
pub struct OutboundControlRequest<'a> {
    /// Always `"control_request"`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// Our correlation id.
    pub request_id: &'a str,
    /// The request body.
    pub request: OutboundRequestBody,
}

/// Body of an outbound control request, tagged by `subtype`.
#[derive(Debug, Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum OutboundRequestBody {
    /// Interrupt the current turn.
    Interrupt,
    /// Switch the model mid-session.
    SetModel {
        /// New model id (wire string).
        model: String,
    },
    /// Switch the permission mode mid-session.
    SetPermissionMode {
        /// New mode (wire string).
        mode: String,
    },
    /// Query MCP server status.
    McpStatus,
}

/// An outbound `control_response` answering an inbound control request.
#[derive(Debug, Serialize)]
pub struct OutboundControlResponse {
    /// Always `"control_response"`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// The response body.
    pub response: OutboundResponseBody,
}

/// Body of an outbound control response.
#[derive(Debug, Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum OutboundResponseBody {
    /// Successful response echoing the server's request id.
    Success {
        /// The inbound request's id.
        request_id: String,
        /// Structured response payload.
        response: serde_json::Value,
    },
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::{ControlResponse, ControlResponseBody, InboundFrame};

    #[test]
    fn classifies_message_frame() {
        let line = r#"{"type":"result","subtype":"success","result":"ok","is_error":false,"session_id":"s1","num_turns":1}"#;
        let frame: InboundFrame = serde_json::from_str(line).expect("deserialize");
        assert!(matches!(frame, InboundFrame::Message(_)));
    }

    #[test]
    fn classifies_control_response_success() {
        let line = r#"{"type":"control_response","response":{"subtype":"success","request_id":"req_1","response":{"ok":true}}}"#;
        let frame: InboundFrame = serde_json::from_str(line).expect("deserialize");
        match frame {
            InboundFrame::ControlResponse(ControlResponse { response }) => match response {
                ControlResponseBody::Success { request_id, .. } => assert_eq!(request_id, "req_1"),
                ControlResponseBody::Error { .. } => panic!("expected success"),
            },
            other => panic!("expected ControlResponse, got {other:?}"),
        }
    }

    #[test]
    fn classifies_inbound_control_request() {
        let line = r#"{"type":"control_request","request_id":"srv_5","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"cmd":"ls"}}}"#;
        let frame: InboundFrame = serde_json::from_str(line).expect("deserialize");
        assert!(matches!(frame, InboundFrame::ControlRequest(_)));
    }
}
