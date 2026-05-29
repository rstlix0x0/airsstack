//! Decoded response types from the Messages API.
//!
//! Exists as its own module so the response envelope and its sub-types
//! can evolve independently of request construction.
//!
//! Responsibilities:
//! - Define [`Message`], the top-level decoded response.
//! - Define [`MessageKind`], [`StopReason`], and [`Usage`] sub-types.
//!
//! Not responsible for:
//! - HTTP transport or envelope unwrapping — those live in `resource.rs`.
//! - Request types — those live in `request.rs`.

use crate::messages::content::ContentBlock;
use crate::messages::request::Role;
use crate::types::{MessageId, ModelId, StopSequence};

/// Decoded top-level response from `POST /v1/messages`.
///
/// # Examples
///
/// ```no_run
/// # use clauders::messages::response::Message;
/// // Typically obtained by calling `client.messages().create(...)`.
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Message {
    /// Server-generated message identifier.
    pub id: MessageId,
    /// Wire-format type discriminant — always `MessageKind::Message`.
    #[serde(rename = "type")]
    pub kind: MessageKind,
    /// Role of the message author — always `Role::Assistant` for responses.
    pub role: Role,
    /// Model that generated the response.
    pub model: ModelId,
    /// Content blocks that make up the response body.
    pub content: Vec<ContentBlock>,
    /// Reason the model stopped generating tokens, if generation is complete.
    pub stop_reason: Option<StopReason>,
    /// The stop sequence that triggered the stop, if applicable.
    pub stop_sequence: Option<StopSequence>,
    /// Token counts for this request-response pair.
    pub usage: Usage,
}

/// Wire-format `type` discriminant for a message response.
///
/// The API always returns `"message"` here; the variant exists so the
/// field is strongly typed rather than an unchecked string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// Standard message response.
    Message,
}

/// Reason the model stopped generating tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Model reached a natural stopping point.
    EndTurn,
    /// Generation stopped because `max_tokens` was reached.
    MaxTokens,
    /// Generation stopped because a stop sequence was matched.
    StopSequence,
}

/// Input and output token counts for a request-response pair.
///
/// # Examples
///
/// ```no_run
/// # use clauders::messages::response::Usage;
/// // Obtained from a decoded Message response.
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Usage {
    /// Number of tokens in the input (prompt + system).
    pub input_tokens: u32,
    /// Number of tokens generated in the response.
    pub output_tokens: u32,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    const MINIMAL_RESPONSE: &str = r#"{
        "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
        "type": "message",
        "role": "assistant",
        "model": "claude-sonnet-4-5",
        "content": [{"type": "text", "text": "Hi there!"}],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {"input_tokens": 25, "output_tokens": 5}
    }"#;

    #[test]
    fn decodes_minimal_messages_response() {
        let msg: Message = serde_json::from_str(MINIMAL_RESPONSE).unwrap();
        assert_eq!(msg.id.as_str(), "msg_01XFDUDYJgAACzvnptvVoYEL");
        assert_eq!(msg.kind, MessageKind::Message);
        assert_eq!(msg.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(msg.usage.input_tokens, 25);
        assert_eq!(msg.usage.output_tokens, 5);
        assert_eq!(msg.content.len(), 1);
    }
}
