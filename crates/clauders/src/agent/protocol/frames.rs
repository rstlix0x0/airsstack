//! Serde types for the control-protocol wire frames.
//!
//! Three inbound frame kinds arrive on the binary's stdout, discriminated by
//! the top-level `type` field:
//! - a **message** frame (`assistant`/`user`/`system`/`result`/`stream_event`)
//!   carrying model output, forwarded to the caller's message stream;
//! - a **`control_response`** replying to one of our outbound control
//!   requests, matched back to its waiter by `request_id`, plus a
//!   `Malformed` fallback [`decode_inbound`](crate::agent::protocol::decode_inbound)
//!   builds by hand when a response body fails to deserialize but its
//!   `request_id` can still be read back out — otherwise the pending waiter
//!   would never resolve and the caller would block until the process exits;
//! - an inbound **`control_request`** (`can_use_tool`/`hook_callback`/
//!   `mcp_message`/`elicitation`, plus an `Other` catch-all for a subtype this
//!   version does not model, and a `Malformed` fallback
//!   [`decode_inbound`](crate::agent::protocol::decode_inbound) builds by
//!   hand when a recognized subtype's body fails to deserialize) the binary
//!   issues to us mid-turn, answered with a correlated control response
//!   either way — the binary blocks on it.
//!
//! Outbound, the runtime writes a user-message frame (the prompt) and
//! `control_request` frames (`interrupt`/`set_model`/…).

use serde::{Deserialize, Serialize};

use crate::agent::elicitation::ElicitationMode;
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
    /// A `control_response` whose body failed to deserialize.
    ///
    /// Built by hand in `decode_inbound`'s rescue path from the raw JSON
    /// value, never produced by serde: `#[serde(skip)]` removes this
    /// variant's tag name from the internally-tagged match serde generates,
    /// so a wire frame carrying it verbatim (e.g. `"subtype":"malformed"`)
    /// still fails typed decode like any other unmodeled `subtype` —
    /// `ControlResponseBody` has no `Other` catch-all, so that failure
    /// always reaches the rescue path rather than silently matching here.
    ///
    /// This is the outbound mirror of
    /// [`InboundRequestBody::Malformed`]: without it, a `control_response`
    /// whose body doesn't match `Success` or `Error` would be rescued into
    /// [`Message::Other`](crate::agent::message::Message::Other) instead,
    /// routed to the caller's message stream, and the pending oneshot
    /// waiting on it would never resolve — the caller would block until the
    /// subprocess exits. Kept distinct from `Error` so callers can tell a
    /// genuine binary-reported failure from a locally-detected decode fault.
    #[serde(skip)]
    Malformed {
        /// Correlation id matching our outbound request.
        request_id: String,
        /// Why the body could not be interpreted.
        detail: String,
    },
}

impl ControlResponseBody {
    /// The correlation id carried by any variant.
    #[must_use]
    pub fn request_id(&self) -> &str {
        match self {
            Self::Success { request_id, .. }
            | Self::Error { request_id, .. }
            | Self::Malformed { request_id, .. } => request_id,
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
        /// Id of the tool-use block this request gates.
        #[serde(default)]
        tool_use_id: Option<String>,
        /// Id of the (sub)agent issuing the call.
        #[serde(default)]
        agent_id: Option<String>,
        /// Path the call is blocked on, when applicable.
        #[serde(default)]
        blocked_path: Option<String>,
        /// The binary's own pre-decision reason.
        #[serde(default)]
        decision_reason: Option<String>,
        /// Short human title.
        #[serde(default)]
        title: Option<String>,
        /// Display name of the tool.
        #[serde(default)]
        display_name: Option<String>,
        /// Longer human description.
        #[serde(default)]
        description: Option<String>,
    },
    /// The binary invokes a registered hook.
    HookCallback {
        /// Hook callback id / event name.
        #[serde(default)]
        callback_id: String,
        /// Hook input payload (opaque).
        #[serde(default)]
        input: serde_json::Value,
        /// The tool-use id in scope, when tool-related.
        #[serde(default)]
        tool_use_id: Option<String>,
    },
    /// The backend forwards a JSON-RPC message to an in-process MCP server.
    McpMessage {
        /// The target in-process server's name.
        server_name: String,
        /// The raw MCP JSON-RPC message.
        #[serde(default)]
        message: serde_json::Value,
    },
    /// An MCP server requests structured input mid-tool-call.
    Elicitation {
        /// Correlation id the binary assigned this elicitation.
        elicitation_id: String,
        /// Human-readable prompt.
        #[serde(default)]
        message: String,
        /// Form-mode or url-mode.
        mode: ElicitationMode,
        /// The requested MCP JSON Schema (form mode).
        #[serde(default)]
        requested_schema: Option<serde_json::Value>,
        /// The URL to visit (url mode).
        #[serde(default)]
        url: Option<String>,
        /// The MCP server that raised the elicitation.
        #[serde(default)]
        mcp_server_name: Option<String>,
        /// Short human title.
        #[serde(default)]
        title: Option<String>,
        /// Display name of the requesting server.
        #[serde(default)]
        display_name: Option<String>,
        /// Longer human description.
        #[serde(default)]
        description: Option<String>,
    },
    /// A known-`type` control request whose body failed to deserialize —
    /// e.g. a required field is missing or wrong-typed.
    ///
    /// Built by hand in `decode_inbound`'s rescue path from the raw JSON
    /// `value`, never produced by serde: `#[serde(skip)]` removes this
    /// variant's tag name from the internally-tagged match serde generates,
    /// so a wire frame carrying it verbatim (e.g. `"subtype":"malformed"`)
    /// still falls through to [`Other`](Self::Other) like any other
    /// unmodeled subtype. Declared before `Other` because `#[serde(other)]`
    /// is only valid on the enum's last variant.
    ///
    /// Kept distinct from `Other` so the dispatcher can report which fault
    /// occurred (a recognized-but-malformed body vs. a wholly unrecognized
    /// subtype) while still always writing a correlated control response —
    /// the binary blocks on `request_id` until one arrives.
    #[serde(skip)]
    Malformed {
        /// The wire `subtype` string, when present and string-typed.
        ///
        /// Named `subtype` to match its meaning, which happens to be the
        /// same name as this enum's `#[serde(tag = "subtype")]`
        /// discriminant. Harmless today because `#[serde(skip)]` removes
        /// this variant from serde's tag dispatch entirely, so the derive
        /// never has to reconcile the two — but a maintainer who later
        /// removes `skip` without also renaming this field would hit a
        /// confusing duplicate-field/tag collision instead of a clean
        /// error. Do not rename: the dispatch arm and tests read against
        /// this exact field name.
        subtype: Option<String>,
    },
    /// A control-request subtype this version does not model.
    ///
    /// Kept so an unrecognized subtype degrades to an error control response
    /// instead of failing the whole turn.
    #[serde(other)]
    Other,
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
    /// Failure response echoing the server's request id.
    Error {
        /// The inbound request's id.
        request_id: String,
        /// Human-readable error detail.
        error: String,
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
                other => panic!("expected success, got {other:?}"),
            },
            other => panic!("expected ControlResponse, got {other:?}"),
        }
    }

    #[test]
    fn classifies_control_response_error() {
        let line = r#"{"type":"control_response","response":{"subtype":"error","request_id":"req_2","error":"denied"}}"#;
        let frame: InboundFrame = serde_json::from_str(line).expect("deserialize");
        match frame {
            InboundFrame::ControlResponse(ControlResponse { response }) => match response {
                ControlResponseBody::Error { request_id, error } => {
                    assert_eq!(request_id, "req_2");
                    assert_eq!(error, "denied");
                }
                other => panic!("expected error, got {other:?}"),
            },
            other => panic!("expected ControlResponse, got {other:?}"),
        }
    }

    #[test]
    fn control_response_malformed_variant_carries_request_id() {
        // Direct construction: this variant is never produced by serde (see
        // `control_response_subtype_malformed_is_not_serde_reachable` below),
        // only hand-built by `decode_inbound`'s rescue path. `request_id()`
        // must still cover it, since that is the accessor the demux routes
        // on to resolve the correlated waiter.
        let body = ControlResponseBody::Malformed {
            request_id: "req_3".to_string(),
            detail: "unmodeled subtype".to_string(),
        };
        assert_eq!(body.request_id(), "req_3");
    }

    #[test]
    fn control_response_subtype_malformed_is_not_serde_reachable() {
        // A wire control_response carrying the literal subtype string
        // "malformed" must not deserialize directly into the hand-built
        // `Malformed` variant via serde's tag dispatch: `#[serde(skip)]`
        // removes it from the internally-tagged match the derive generates,
        // so typed decode fails here exactly as it would for any other
        // unmodeled subtype (`ControlResponseBody` has no `Other` catch-all).
        // Recovering a response like this is `decode_inbound`'s job, not
        // serde's; see the codec-level rescue tests for that path.
        let line = r#"{"type":"control_response","response":{"subtype":"malformed","request_id":"req_4","detail":"nope"}}"#;
        let result = serde_json::from_str::<InboundFrame>(line);
        assert!(
            result.is_err(),
            "expected typed decode to fail, got {result:?}"
        );
    }

    #[test]
    fn classifies_inbound_control_request() {
        let line = r#"{"type":"control_request","request_id":"srv_5","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"cmd":"ls"}}}"#;
        let frame: InboundFrame = serde_json::from_str(line).expect("deserialize");
        assert!(matches!(frame, InboundFrame::ControlRequest(_)));
    }

    #[test]
    fn serializes_error_control_response() {
        use super::{OutboundControlResponse, OutboundResponseBody};
        let frame = OutboundControlResponse {
            kind: "control_response",
            response: OutboundResponseBody::Error {
                request_id: "srv_7".to_string(),
                error: "handler failed".to_string(),
            },
        };
        let value = serde_json::to_value(frame).expect("serialize");
        assert_eq!(value["type"], "control_response");
        assert_eq!(value["response"]["subtype"], "error");
        assert_eq!(value["response"]["request_id"], "srv_7");
        assert_eq!(value["response"]["error"], "handler failed");
    }

    #[test]
    fn deserializes_can_use_tool_context_fields() {
        use super::{InboundFrame, InboundRequestBody};
        let line = r#"{"type":"control_request","request_id":"srv_8","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"cmd":"ls"},"tool_use_id":"tu_1","blocked_path":"/etc"}}"#;
        let frame: InboundFrame = serde_json::from_str(line).expect("deserialize");
        let InboundFrame::ControlRequest(req) = frame else {
            panic!("expected control request");
        };
        let InboundRequestBody::CanUseTool {
            tool_name,
            tool_use_id,
            blocked_path,
            ..
        } = req.request
        else {
            panic!("expected can_use_tool");
        };
        assert_eq!(tool_name, "Bash");
        assert_eq!(tool_use_id.as_deref(), Some("tu_1"));
        assert_eq!(blocked_path.as_deref(), Some("/etc"));
    }

    #[test]
    fn deserializes_mcp_message_request() {
        use super::{InboundFrame, InboundRequestBody};
        let line = r#"{"type":"control_request","request_id":"srv_9","request":{"subtype":"mcp_message","server_name":"calc","message":{"jsonrpc":"2.0","id":1,"method":"tools/list"}}}"#;
        let frame: InboundFrame = serde_json::from_str(line).expect("deserialize");
        let InboundFrame::ControlRequest(req) = frame else {
            panic!("expected control request");
        };
        let InboundRequestBody::McpMessage {
            server_name,
            message,
        } = req.request
        else {
            panic!("expected mcp_message");
        };
        assert_eq!(server_name, "calc");
        assert_eq!(message["method"], "tools/list");
    }

    #[test]
    fn decodes_form_mode_elicitation_request() {
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        let line = r#"{"type":"control_request","request_id":"srv_11","request":{"subtype":"elicitation","elicitation_id":"elic_1","message":"Pick a branch","mode":"form","requested_schema":{"type":"object"},"mcp_server_name":"git","title":"Branch","display_name":"Git","description":"Choose"}}"#;
        let frame = decode_inbound(line).expect("decode");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        let InboundRequestBody::Elicitation {
            elicitation_id,
            message,
            mode,
            requested_schema,
            mcp_server_name,
            display_name,
            ..
        } = req.request
        else {
            unreachable!("decoded an elicitation request")
        };
        assert_eq!(elicitation_id, "elic_1");
        assert_eq!(message, "Pick a branch");
        assert_eq!(mode, crate::agent::elicitation::ElicitationMode::Form);
        assert_eq!(requested_schema.expect("schema")["type"], "object");
        assert_eq!(mcp_server_name.as_deref(), Some("git"));
        assert_eq!(display_name.as_deref(), Some("Git"));
    }

    #[test]
    fn decodes_url_mode_elicitation_request() {
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        let line = r#"{"type":"control_request","request_id":"srv_12","request":{"subtype":"elicitation","elicitation_id":"elic_2","message":"Authorize","mode":"url","url":"https://example.test/auth"}}"#;
        let frame = decode_inbound(line).expect("decode");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        let InboundRequestBody::Elicitation { mode, url, .. } = req.request else {
            unreachable!("decoded an elicitation request")
        };
        assert_eq!(mode, crate::agent::elicitation::ElicitationMode::Url);
        assert_eq!(url.as_deref(), Some("https://example.test/auth"));
    }

    #[test]
    fn unknown_elicitation_mode_still_reaches_dispatcher() {
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        let line = r#"{"type":"control_request","request_id":"srv_14","request":{"subtype":"elicitation","elicitation_id":"elic_3","message":"Confirm?","mode":"confirm"}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        let InboundRequestBody::Elicitation { mode, .. } = req.request else {
            unreachable!("decoded an elicitation request")
        };
        assert_eq!(mode, crate::agent::elicitation::ElicitationMode::Unknown);
    }

    #[test]
    fn unknown_control_request_subtype_decodes_to_other() {
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        let line = r#"{"type":"control_request","request_id":"srv_13","request":{"subtype":"some_future_subtype","whatever":1}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        assert!(matches!(req.request, InboundRequestBody::Other));
    }

    #[test]
    fn elicitation_missing_mode_rescues_to_malformed() {
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        let line = r#"{"type":"control_request","request_id":"srv_15","request":{"subtype":"elicitation","elicitation_id":"elic_4","message":"Pick"}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        assert!(matches!(
            req.request,
            InboundRequestBody::Malformed {
                subtype: Some(ref s)
            } if s == "elicitation"
        ));
    }

    #[test]
    fn elicitation_wrong_typed_mode_rescues_to_malformed() {
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        let line = r#"{"type":"control_request","request_id":"srv_16","request":{"subtype":"elicitation","elicitation_id":"elic_5","message":"Pick","mode":42}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        assert!(matches!(
            req.request,
            InboundRequestBody::Malformed {
                subtype: Some(ref s)
            } if s == "elicitation"
        ));
    }

    #[test]
    fn elicitation_missing_elicitation_id_rescues_to_malformed() {
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        let line = r#"{"type":"control_request","request_id":"srv_17","request":{"subtype":"elicitation","message":"Pick","mode":"form"}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        assert!(matches!(
            req.request,
            InboundRequestBody::Malformed {
                subtype: Some(ref s)
            } if s == "elicitation"
        ));
    }

    #[test]
    fn malformed_can_use_tool_rescues_to_malformed_with_its_own_subtype() {
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        // Missing the required `tool_name` field.
        let line = r#"{"type":"control_request","request_id":"srv_18","request":{"subtype":"can_use_tool","input":{"cmd":"ls"}}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        assert!(matches!(
            req.request,
            InboundRequestBody::Malformed {
                subtype: Some(ref s)
            } if s == "can_use_tool"
        ));
    }

    #[test]
    fn subtype_string_malformed_is_not_serde_reachable() {
        // A wire frame that happens to carry the literal subtype string
        // "malformed" must still land in `Other`: `Malformed` is a
        // decode-time fallback variant, never a serde-reachable target.
        use super::InboundRequestBody;
        use crate::agent::protocol::decode_inbound;

        let line =
            r#"{"type":"control_request","request_id":"srv_19","request":{"subtype":"malformed"}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlRequest(req) = frame else {
            unreachable!("decoded a control request")
        };
        assert!(
            matches!(req.request, InboundRequestBody::Other),
            "expected Other, got {:?}",
            req.request
        );
    }
}
