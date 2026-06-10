//! Line framing and (de)serialization for the control protocol.
//!
//! Inbound lines (already newline-split by the process layer's `StdoutLines`)
//! are parsed into an [`InboundFrame`]; outbound frames are serialized to a
//! single newline-terminated JSON line. [`RequestId`] mints the correlation
//! ids that match an outbound control request to its response.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;

use crate::agent::error::AgentError;
use crate::agent::protocol::frames::InboundFrame;

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
/// # Errors
/// Returns [`AgentError::Decode`] if the line is not valid JSON or does not
/// match any known frame shape.
pub fn decode_inbound(line: &str) -> Result<InboundFrame, AgentError> {
    serde_json::from_str(line).map_err(|e| AgentError::Decode(e.to_string()))
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

    use super::{decode_inbound, encode_line, RequestId};
    use crate::agent::protocol::frames::{InboundFrame, OutboundControlRequest, OutboundRequestBody};

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
        assert!(shown.contains("decode") || shown.contains("protocol"), "got {shown}");
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
        assert_eq!(line.matches('\n').count(), 1, "exactly one trailing newline");
        assert!(line.contains("\"subtype\":\"interrupt\""));
    }
}
