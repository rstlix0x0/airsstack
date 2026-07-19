//! Line framing and (de)serialization for the control protocol.
//!
//! Inbound lines (already newline-split by the process layer's `StdoutLines`)
//! are parsed into an [`InboundFrame`]; outbound frames are serialized to a
//! single newline-terminated JSON line. [`RequestId`] mints the correlation
//! ids that match an outbound control request to its response.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::agent::error::AgentError;
use crate::agent::message::Message;
use crate::agent::protocol::frames::{
    ControlResponse, ControlResponseBody, InboundControlRequest, InboundFrame, InboundRequestBody,
};

/// A control-request correlation id (`req_<n>`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RequestId(String);

impl RequestId {
    /// Create a process-local, monotonic id generator.
    #[must_use]
    pub fn generator() -> RequestIdGen {
        RequestIdGen {
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Borrow the id string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Monotonic generator of unique [`RequestId`]s, cheap to clone (shared counter).
#[derive(Clone, Debug)]
pub struct RequestIdGen {
    counter: Arc<AtomicU64>,
}

impl RequestIdGen {
    /// Mint the next unique id.
    #[must_use]
    pub fn next(&self) -> RequestId {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        RequestId(format!("req_{n}"))
    }
}

/// Parse one inbound line into a frame.
///
/// A structurally-valid JSON line that matches no known frame shape — or a
/// known-`type` frame other than `control_request`/`control_response` whose
/// payload fails to deserialize — is captured verbatim as [`Message::Other`]
/// rather than erroring, keeping the stdout stream forward-compatible. That
/// path is logged at `debug`: it is expected whenever the wire carries a shape
/// this version does not model yet, so logging it louder would drown the
/// `warn`-level control-frame rescues below, which indicate real defects.
///
/// A `control_request` or `control_response` whose body fails to deserialize
/// is handled differently: either direction leaves something blocked on a
/// `request_id`-correlated response (the binary blocks on an unanswered
/// `control_request`; our own caller blocks on an unresolved `control_response`
/// waiter), so silently rescuing either into `Message::Other` — which nothing
/// ever answers or resolves — would hang forever. When the relevant
/// `request_id` is present and string-typed, this instead rescues the line to
/// [`InboundFrame::ControlRequest`] carrying [`InboundRequestBody::Malformed`]
/// (answered by the dispatcher with an error control response like any other
/// handler fault) or to [`InboundFrame::ControlResponse`] carrying
/// [`ControlResponseBody::Malformed`] (resolving the pending waiter with an
/// error instead of leaving it pending). Only when no `request_id` can be
/// recovered — leaving nothing to correlate a response to — does either kind
/// of malformed control frame fall back to `Message::Other`.
///
/// # Errors
/// Returns [`AgentError::Decode`] only when the line is not valid JSON.
pub fn decode_inbound(line: &str) -> Result<InboundFrame, AgentError> {
    serde_json::from_str::<InboundFrame>(line).or_else(|frame_err| {
        // Unknown frame type, or a known type whose payload fails to
        // deserialize: capture structurally-valid JSON verbatim as a
        // catch-all message rather than failing the turn, unless it can be
        // rescued as a malformed control request or response instead.
        // Genuinely malformed lines remain decode errors.
        serde_json::from_str::<serde_json::Value>(line).map_or_else(
            |_| Err(AgentError::Decode(frame_err.to_string())),
            |value| Ok(rescue_frame(value, &frame_err)),
        )
    })
}

/// Recover a frame that failed typed decode, in priority order: a malformed
/// `control_request`, then a malformed `control_response`, then — when
/// neither rescue applies — the raw JSON captured verbatim as
/// [`Message::Other`].
fn rescue_frame(value: serde_json::Value, frame_err: &serde_json::Error) -> InboundFrame {
    if let Some(frame) = rescue_malformed_control_request(&value, frame_err) {
        return frame;
    }
    if let Some(frame) = rescue_malformed_control_response(&value, frame_err) {
        return frame;
    }
    // `debug`, not `warn`: this is the documented forward-compatibility path,
    // and it fires for every frame shape we do not model yet — under
    // `include_hook_events(true)` that is one per hook-lifecycle frame. Warning
    // here would drown the two rescue warnings above, which are real defects.
    tracing::debug!(
        frame_type = value.get("type").and_then(serde_json::Value::as_str),
        error = %frame_err,
        "frame failed typed decode; captured verbatim as Message::Other"
    );
    InboundFrame::Message(Message::Other(value))
}

/// Recover a `control_request` frame whose body failed to deserialize into
/// [`InboundRequestBody`], provided its `request_id` can be read back out of
/// the raw JSON.
///
/// Returns `None` when `value` is not a `control_request` frame, or when its
/// `request_id` is absent or not a string — the forced limit documented on
/// [`decode_inbound`].
fn rescue_malformed_control_request(
    value: &serde_json::Value,
    frame_err: &serde_json::Error,
) -> Option<InboundFrame> {
    if value.get("type").and_then(serde_json::Value::as_str) != Some("control_request") {
        return None;
    }
    let request_id = value.get("request_id")?.as_str()?.to_string();
    let subtype = value
        .get("request")
        .and_then(|request| request.get("subtype"))
        .and_then(serde_json::Value::as_str)
        .map(String::from);
    tracing::warn!(
        subtype,
        request_id = %request_id,
        error = %frame_err,
        "rescued malformed control_request body"
    );
    Some(InboundFrame::ControlRequest(InboundControlRequest {
        request_id,
        request: InboundRequestBody::Malformed { subtype },
    }))
}

/// Recover a `control_response` frame whose body failed to deserialize into
/// [`ControlResponseBody`], provided its `request_id` can be read back out of
/// the raw JSON.
///
/// Returns `None` when `value` is not a `control_response` frame, or when its
/// `response.request_id` is absent or not a string — the forced limit
/// documented on [`decode_inbound`], mirroring the inbound `control_request`
/// rescue above.
fn rescue_malformed_control_response(
    value: &serde_json::Value,
    frame_err: &serde_json::Error,
) -> Option<InboundFrame> {
    if value.get("type").and_then(serde_json::Value::as_str) != Some("control_response") {
        return None;
    }
    let response = value.get("response")?;
    let request_id = response.get("request_id")?.as_str()?.to_string();
    let subtype = response
        .get("subtype")
        .and_then(serde_json::Value::as_str)
        .map(String::from);
    // Name the subtype the way the inbound rescue does — the raw serde error
    // for an untagged enum ("data did not match any variant of ...") tells the
    // caller nothing about which response shape actually failed.
    let detail = format!(
        "malformed control_response (subtype: {}): {frame_err}",
        subtype.as_deref().unwrap_or("absent")
    );
    tracing::warn!(
        subtype,
        request_id = %request_id,
        error = %frame_err,
        "rescued malformed control_response body"
    );
    Some(InboundFrame::ControlResponse(ControlResponse {
        response: ControlResponseBody::Malformed { request_id, detail },
    }))
}

/// Serialize an outbound frame to a single newline-terminated JSON line.
///
/// # Errors
/// Returns [`AgentError::Protocol`] if the value cannot be serialized.
pub fn encode_line<T: Serialize>(frame: &T) -> Result<String, AgentError> {
    let mut line = serde_json::to_string(frame).map_err(|e| AgentError::Protocol {
        detail: format!("failed to serialize outbound frame: {e}"),
    })?;
    line.push('\n');
    Ok(line)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::{RequestId, decode_inbound, encode_line};
    use crate::agent::protocol::frames::{
        InboundFrame, OutboundControlRequest, OutboundRequestBody,
    };

    #[test]
    fn request_ids_are_unique_and_prefixed() {
        let id_gen = RequestId::generator();
        let a = id_gen.next();
        let b = id_gen.next();
        assert_ne!(a.as_str(), b.as_str());
        assert!(a.as_str().starts_with("req_"), "got {}", a.as_str());
    }

    #[test]
    fn decodes_valid_line_to_frame() {
        let line = r#"{"type":"result","subtype":"success","result":"ok","is_error":false,"session_id":"s1","num_turns":1}"#;
        let frame = decode_inbound(line).expect("decode");
        assert!(matches!(frame, InboundFrame::Message(_)));
    }

    #[test]
    fn malformed_line_is_protocol_error() {
        let err = decode_inbound("{ not json").expect_err("should fail");
        let shown = err.to_string();
        assert!(
            shown.contains("decode") || shown.contains("protocol"),
            "got {shown}"
        );
    }

    #[test]
    fn encodes_frame_as_single_newline_terminated_line() {
        let req = OutboundControlRequest {
            kind: "control_request",
            request_id: "req_9",
            request: OutboundRequestBody::Interrupt,
        };
        let line = encode_line(&req).expect("encode");
        assert!(line.ends_with('\n'));
        assert_eq!(
            line.matches('\n').count(),
            1,
            "exactly one trailing newline"
        );
        assert!(line.contains("\"subtype\":\"interrupt\""));
    }

    #[test]
    fn unknown_frame_type_decodes_to_message_other_with_payload() {
        use crate::agent::message::Message;
        use crate::agent::protocol::frames::InboundFrame;
        // A hook-lifecycle frame the enum has no typed variant for.
        let line = r#"{"type":"hook_started","hook":"SessionStart","session_id":"s1"}"#;
        let frame = decode_inbound(line).expect("decode");
        match frame {
            InboundFrame::Message(Message::Other(v)) => {
                assert_eq!(v["type"], "hook_started");
                assert_eq!(v["hook"], "SessionStart");
            }
            other => panic!("expected Message::Other, got {other:?}"),
        }
    }

    #[test]
    fn known_frame_types_still_decode_to_their_variants() {
        use crate::agent::message::Message;
        use crate::agent::protocol::frames::InboundFrame;
        let line = r#"{"type":"result","subtype":"success","result":"ok","is_error":false,"session_id":"s1","num_turns":1}"#;
        assert!(matches!(
            decode_inbound(line).expect("decode"),
            InboundFrame::Message(Message::Result(_))
        ));
    }

    #[test]
    fn control_request_with_no_subtype_rescues_to_malformed_with_none() {
        use crate::agent::protocol::frames::{InboundFrame, InboundRequestBody};

        let line = r#"{"type":"control_request","request_id":"srv_30","request":{}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlRequest(req) = frame else {
            panic!("expected ControlRequest, got {frame:?}")
        };
        assert!(
            matches!(req.request, InboundRequestBody::Malformed { subtype: None }),
            "expected Malformed {{ subtype: None }}, got {:?}",
            req.request
        );
    }

    #[test]
    fn control_request_missing_request_id_falls_back_to_message_other() {
        // With no id there is nothing to correlate a control response to, so
        // this can never be rescued into a `ControlRequest` — it falls
        // through to the same catch-all as any other unmodeled frame shape.
        use crate::agent::message::Message;
        use crate::agent::protocol::frames::InboundFrame;

        let line = r#"{"type":"control_request","request":{"subtype":"elicitation"}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        match frame {
            InboundFrame::Message(Message::Other(v)) => {
                assert_eq!(v["type"], "control_request");
            }
            other => panic!("expected Message::Other, got {other:?}"),
        }
    }

    #[test]
    fn control_request_request_id_non_string_falls_back_to_message_other() {
        // A `request_id` present but wrong-typed is just as uncorrelatable as
        // one that's absent — pins the `.as_str()?` half of the forced limit,
        // not just the `.get("request_id")?` half.
        use crate::agent::message::Message;
        use crate::agent::protocol::frames::InboundFrame;

        let line =
            r#"{"type":"control_request","request_id":42,"request":{"subtype":"elicitation"}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        match frame {
            InboundFrame::Message(Message::Other(v)) => {
                assert_eq!(v["type"], "control_request");
            }
            other => panic!("expected Message::Other, got {other:?}"),
        }
    }

    #[test]
    fn control_response_with_unmodeled_subtype_rescues_to_malformed_with_detail() {
        use crate::agent::protocol::frames::{ControlResponseBody, InboundFrame};

        let line = r#"{"type":"control_response","response":{"subtype":"some_future_subtype","request_id":"req_40"}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        let InboundFrame::ControlResponse(resp) = frame else {
            panic!("expected ControlResponse, got {frame:?}")
        };
        match resp.response {
            ControlResponseBody::Malformed { request_id, detail } => {
                assert_eq!(request_id, "req_40");
                assert!(!detail.is_empty(), "detail should carry the decode error");
            }
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn control_response_missing_request_id_falls_back_to_message_other() {
        // With no id there is nothing to correlate the pending waiter to, so
        // this can never be rescued into a `ControlResponse` — it falls
        // through to the same catch-all as any other unmodeled frame shape.
        use crate::agent::message::Message;
        use crate::agent::protocol::frames::InboundFrame;

        let line = r#"{"type":"control_response","response":{"subtype":"some_future_subtype"}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        match frame {
            InboundFrame::Message(Message::Other(v)) => {
                assert_eq!(v["type"], "control_response");
            }
            other => panic!("expected Message::Other, got {other:?}"),
        }
    }

    #[test]
    fn control_response_request_id_non_string_falls_back_to_message_other() {
        // Mirrors `control_request_request_id_non_string_falls_back_to_message_other`:
        // a wrong-typed `request_id` is just as uncorrelatable as an absent one.
        use crate::agent::message::Message;
        use crate::agent::protocol::frames::InboundFrame;

        let line = r#"{"type":"control_response","response":{"subtype":"some_future_subtype","request_id":42}}"#;
        let frame = decode_inbound(line).expect("decode must not fail");
        match frame {
            InboundFrame::Message(Message::Other(v)) => {
                assert_eq!(v["type"], "control_response");
            }
            other => panic!("expected Message::Other, got {other:?}"),
        }
    }

    #[test]
    fn well_formed_control_response_success_and_error_still_decode_as_before() {
        use crate::agent::protocol::frames::{ControlResponseBody, InboundFrame};

        let success_line = r#"{"type":"control_response","response":{"subtype":"success","request_id":"req_41","response":{"ok":true}}}"#;
        let InboundFrame::ControlResponse(resp) =
            decode_inbound(success_line).expect("decode success")
        else {
            panic!("expected ControlResponse")
        };
        assert!(matches!(resp.response, ControlResponseBody::Success { .. }));

        let error_line = r#"{"type":"control_response","response":{"subtype":"error","request_id":"req_42","error":"denied"}}"#;
        let InboundFrame::ControlResponse(resp) = decode_inbound(error_line).expect("decode error")
        else {
            panic!("expected ControlResponse")
        };
        assert!(matches!(resp.response, ControlResponseBody::Error { .. }));
    }
}
