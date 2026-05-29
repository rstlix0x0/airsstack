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
    /// Model stopped to call one or more tools.
    #[cfg(feature = "messages-tools")]
    ToolUse,
    /// Model declined to produce the constrained output.
    #[cfg(feature = "messages-structured-outputs")]
    Refusal,
}

/// Breakdown of tokens created in the cache during a caching-enabled request.
///
/// The sum `ephemeral_5m_input_tokens + ephemeral_1h_input_tokens` equals
/// `Usage::cache_creation_input_tokens`.
///
/// Requires the `messages-caching` feature.
#[cfg(feature = "messages-caching")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-caching")))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct CacheCreation {
    /// Tokens written into the 5-minute ephemeral cache tier.
    pub ephemeral_5m_input_tokens: u32,
    /// Tokens written into the 1-hour ephemeral cache tier.
    pub ephemeral_1h_input_tokens: u32,
}

/// Input and output token counts for a request-response pair.
///
/// When the `messages-caching` feature is enabled, the additional
/// `cache_creation_input_tokens`, `cache_read_input_tokens`, and
/// `cache_creation` fields are populated from caching-aware responses.
/// Use [`Usage::total_input_tokens`] to obtain the full input-side total
/// (regular + cache creation + cache read).
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
    /// Tokens written into the cache during this request.
    ///
    /// Requires the `messages-caching` feature.
    #[cfg(feature = "messages-caching")]
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u32>,
    /// Tokens read from the cache during this request.
    ///
    /// Requires the `messages-caching` feature.
    #[cfg(feature = "messages-caching")]
    #[serde(default)]
    pub cache_read_input_tokens: Option<u32>,
    /// Per-tier breakdown of cache-creation token counts.
    ///
    /// Requires the `messages-caching` feature.
    #[cfg(feature = "messages-caching")]
    #[serde(default)]
    pub cache_creation: Option<CacheCreation>,
}

impl Usage {
    /// Total input-side tokens: regular input + cache-creation + cache-read.
    ///
    /// Returns `input_tokens` when no cache fields are present (all count as 0).
    /// Addition is saturating to guard against malformed server values.
    ///
    /// Requires the `messages-caching` feature.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "messages-caching")] {
    /// use clauders::messages::response::Usage;
    /// let u: Usage = serde_json::from_str(r#"{
    ///     "input_tokens": 100,
    ///     "output_tokens": 5,
    ///     "cache_creation_input_tokens": 200,
    ///     "cache_read_input_tokens": 50
    /// }"#).unwrap();
    /// assert_eq!(u.total_input_tokens(), 350);
    /// # }
    /// ```
    #[cfg(feature = "messages-caching")]
    #[cfg_attr(docsrs, doc(cfg(feature = "messages-caching")))]
    #[must_use]
    pub fn total_input_tokens(&self) -> u32 {
        self.input_tokens
            .saturating_add(self.cache_creation_input_tokens.unwrap_or(0))
            .saturating_add(self.cache_read_input_tokens.unwrap_or(0))
    }
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

    #[cfg(feature = "messages-caching")]
    #[test]
    fn usage_decodes_cache_fields() {
        let j = r#"{"input_tokens":100,"output_tokens":5,"cache_creation_input_tokens":200,"cache_read_input_tokens":50}"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(u.cache_creation_input_tokens, Some(200));
        assert_eq!(u.cache_read_input_tokens, Some(50));
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn usage_without_cache_fields_defaults_to_none() {
        let j = r#"{"input_tokens":10,"output_tokens":5}"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(u.cache_creation_input_tokens, None);
        assert_eq!(u.cache_read_input_tokens, None);
        assert_eq!(u.cache_creation, None);
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn total_input_tokens_sums_all_input_counts() {
        let j = r#"{"input_tokens":100,"output_tokens":5,"cache_creation_input_tokens":200,"cache_read_input_tokens":50}"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(u.total_input_tokens(), 350);
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn total_input_tokens_without_cache_equals_input_tokens() {
        let j = r#"{"input_tokens":42,"output_tokens":5}"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(u.total_input_tokens(), 42);
    }

    #[cfg(feature = "messages-structured-outputs")]
    #[test]
    fn refusal_stop_reason_decodes() {
        let j = r#"{
            "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5",
            "content": [],
            "stop_reason": "refusal",
            "stop_sequence": null,
            "usage": {"input_tokens": 10, "output_tokens": 0}
        }"#;
        let msg: Message = serde_json::from_str(j).unwrap();
        assert_eq!(msg.stop_reason, Some(StopReason::Refusal));
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn cache_creation_breakdown_decodes() {
        let j = r#"{
            "input_tokens":10,
            "output_tokens":2,
            "cache_creation_input_tokens":300,
            "cache_read_input_tokens":0,
            "cache_creation":{"ephemeral_5m_input_tokens":100,"ephemeral_1h_input_tokens":200}
        }"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        let cc = u.cache_creation.unwrap();
        assert_eq!(cc.ephemeral_5m_input_tokens, 100);
        assert_eq!(cc.ephemeral_1h_input_tokens, 200);
    }
}
