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
    /// Structured refusal diagnostic; `Some` only when `stop_reason` is refusal.
    #[serde(default)]
    pub stop_details: Option<StopDetails>,
    /// Code-execution container metadata, when the request used one.
    #[serde(default)]
    pub container: Option<Container>,
    /// Token counts for this request-response pair.
    pub usage: Usage,
}

/// Wire-format `type` discriminant for a message response.
///
/// The API returns `"message"` here today; the variant exists so the
/// field is strongly typed rather than an unchecked string.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MessageKind {
    /// Standard message response.
    Message,
    /// A discriminant this SDK release does not recognize; carries the raw
    /// wire value so it can be inspected or logged.
    #[serde(untagged)]
    Unknown(String),
}

/// Reason the model stopped generating tokens.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StopReason {
    /// Model reached a natural stopping point.
    EndTurn,
    /// Generation stopped because `max_tokens` was reached.
    MaxTokens,
    /// Generation stopped because a stop sequence was matched.
    StopSequence,
    /// Model stopped to call one or more tools.
    ToolUse,
    /// Model declined to produce the constrained output.
    Refusal,
    /// A server-tool loop paused after reaching its iteration limit; the
    /// caller resumes by sending the response back as the next turn.
    PauseTurn,
    /// A stop reason this SDK release does not recognize; carries the raw
    /// wire value so it can be inspected or logged.
    #[serde(untagged)]
    Unknown(String),
}

/// Structured information about why the model refused to answer.
///
/// Present only when `stop_reason` is `refusal`; `None` for every other
/// stop reason. Callers must branch on `stop_reason`, never on the
/// presence of `stop_details` — `explanation` is documented as unstable.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct StopDetails {
    /// Wire-format discriminant — always `refusal` today.
    #[serde(rename = "type")]
    pub kind: StopDetailsKind,
    /// Policy category that triggered the refusal, if the server supplied one.
    #[serde(default)]
    pub category: Option<RefusalCategory>,
    /// Human-readable, non-stable explanation of the refusal.
    #[serde(default)]
    pub explanation: Option<String>,
}

/// Discriminant for [`StopDetails`].
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StopDetailsKind {
    /// A refusal stop-details object.
    Refusal,
    /// A discriminant this SDK release does not recognize; carries the raw
    /// wire value.
    #[serde(untagged)]
    Unknown(String),
}

/// Policy category that triggered a refusal.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RefusalCategory {
    /// Cyber-offense policy category.
    Cyber,
    /// Biological-risk policy category.
    Bio,
    /// Frontier-LLM policy category.
    FrontierLlm,
    /// Reasoning-extraction policy category.
    ReasoningExtraction,
    /// A category this SDK release does not recognize; carries the raw
    /// wire value.
    #[serde(untagged)]
    Unknown(String),
}

/// Information about the code-execution container used in the request.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Container {
    /// Identifier for the container used in this request.
    pub id: String,
    /// RFC 3339 timestamp at which the container expires, kept as a
    /// `String` for forward compatibility with format variation.
    pub expires_at: String,
}

/// Breakdown of tokens created in the cache during a caching-enabled request.
///
/// The sum `ephemeral_5m_input_tokens + ephemeral_1h_input_tokens` equals
/// `Usage::cache_creation_input_tokens`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct CacheCreation {
    /// Tokens written into the 5-minute ephemeral cache tier.
    pub ephemeral_5m_input_tokens: u32,
    /// Tokens written into the 1-hour ephemeral cache tier.
    pub ephemeral_1h_input_tokens: u32,
}

/// Read-only decomposition of `output_tokens` for observability.
///
/// `output_tokens` remains the authoritative billed total; this breaks out
/// how many of those tokens were internal reasoning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct OutputTokensDetails {
    /// Output tokens the model spent on internal reasoning.
    pub thinking_tokens: u32,
}

/// Counts of server-side tool invocations billed to this request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ServerToolUse {
    /// Number of web-search tool requests.
    pub web_search_requests: u32,
    /// Number of web-fetch tool requests.
    pub web_fetch_requests: u32,
}

/// The service tier that actually served the request.
///
/// Distinct from the request-side service-tier selector — this reports what
/// the server used (`standard` / `priority` / `batch`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UsageServiceTier {
    /// Standard tier.
    Standard,
    /// Priority tier.
    Priority,
    /// Batch tier.
    Batch,
    /// A tier this SDK release does not recognize; carries the raw wire value.
    #[serde(untagged)]
    Unknown(String),
}

/// Input and output token counts for a request-response pair.
///
/// The `cache_creation_input_tokens`, `cache_read_input_tokens`, and
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Usage {
    /// Number of tokens in the input (prompt + system).
    pub input_tokens: u32,
    /// Number of tokens generated in the response.
    pub output_tokens: u32,
    /// Tokens written into the cache during this request.
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u32>,
    /// Tokens read from the cache during this request.
    #[serde(default)]
    pub cache_read_input_tokens: Option<u32>,
    /// Per-tier breakdown of cache-creation token counts.
    #[serde(default)]
    pub cache_creation: Option<CacheCreation>,
    /// Read-only decomposition of `output_tokens`.
    #[serde(default)]
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// Counts of server-side tool invocations billed to this request.
    #[serde(default)]
    pub server_tool_use: Option<ServerToolUse>,
    /// The service tier that served this request.
    #[serde(default)]
    pub service_tier: Option<UsageServiceTier>,
    /// Geographic region where inference was performed.
    #[serde(default)]
    pub inference_geo: Option<String>,
}

impl Usage {
    /// Total input-side tokens: regular input + cache-creation + cache-read.
    ///
    /// Returns `input_tokens` when no cache fields are present (all count as 0).
    /// Addition is saturating to guard against malformed server values.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::messages::response::Usage;
    /// let u: Usage = serde_json::from_str(r#"{
    ///     "input_tokens": 100,
    ///     "output_tokens": 5,
    ///     "cache_creation_input_tokens": 200,
    ///     "cache_read_input_tokens": 50
    /// }"#).unwrap();
    /// assert_eq!(u.total_input_tokens(), 350);
    /// ```
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

    #[test]
    fn message_decodes_stop_details_and_container() {
        let j = r#"{
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5",
            "content": [],
            "stop_reason": "refusal",
            "stop_sequence": null,
            "stop_details": {"type":"refusal","category":"bio","explanation":null},
            "container": {"id":"c1","expires_at":"2026-07-22T00:00:00Z"},
            "usage": {"input_tokens": 3, "output_tokens": 0}
        }"#;
        let msg: Message = serde_json::from_str(j).unwrap();
        assert_eq!(
            msg.stop_details.map(|d| d.category),
            Some(Some(RefusalCategory::Bio))
        );
        assert_eq!(msg.container.map(|c| c.id), Some("c1".to_owned()));
    }

    #[test]
    fn usage_decodes_cache_fields() {
        let j = r#"{"input_tokens":100,"output_tokens":5,"cache_creation_input_tokens":200,"cache_read_input_tokens":50}"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(u.cache_creation_input_tokens, Some(200));
        assert_eq!(u.cache_read_input_tokens, Some(50));
    }

    #[test]
    fn usage_without_cache_fields_defaults_to_none() {
        let j = r#"{"input_tokens":10,"output_tokens":5}"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(u.cache_creation_input_tokens, None);
        assert_eq!(u.cache_read_input_tokens, None);
        assert_eq!(u.cache_creation, None);
    }

    #[test]
    fn total_input_tokens_sums_all_input_counts() {
        let j = r#"{"input_tokens":100,"output_tokens":5,"cache_creation_input_tokens":200,"cache_read_input_tokens":50}"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(u.total_input_tokens(), 350);
    }

    #[test]
    fn total_input_tokens_without_cache_equals_input_tokens() {
        let j = r#"{"input_tokens":42,"output_tokens":5}"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(u.total_input_tokens(), 42);
    }

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

    #[test]
    fn pause_turn_stop_reason_decodes_as_typed_variant() {
        let j = r#"{
            "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5",
            "content": [],
            "stop_reason": "pause_turn",
            "stop_sequence": null,
            "usage": {"input_tokens": 10, "output_tokens": 0}
        }"#;
        let msg: Message = serde_json::from_str(j).unwrap();
        assert_eq!(msg.stop_reason, Some(StopReason::PauseTurn));
    }

    #[test]
    fn known_stop_reasons_still_decode_with_unknown_arm_present() {
        let r: StopReason = serde_json::from_str(r#""max_tokens""#).unwrap();
        assert_eq!(r, StopReason::MaxTokens);
    }

    #[test]
    fn unknown_message_kind_decodes_as_unknown_with_payload() {
        let k: MessageKind = serde_json::from_str(r#""some_future_kind""#).unwrap();
        assert_eq!(k, MessageKind::Unknown("some_future_kind".into()));
    }

    #[test]
    fn stop_details_refusal_decodes() {
        let j = r#"{"type":"refusal","category":"cyber","explanation":"nope"}"#;
        let d: StopDetails = serde_json::from_str(j).unwrap();
        assert_eq!(d.kind, StopDetailsKind::Refusal);
        assert_eq!(d.category, Some(RefusalCategory::Cyber));
        assert_eq!(d.explanation.as_deref(), Some("nope"));
    }

    #[test]
    fn stop_details_null_category_and_explanation_decode() {
        let j = r#"{"type":"refusal","category":null,"explanation":null}"#;
        let d: StopDetails = serde_json::from_str(j).unwrap();
        assert_eq!(d.category, None);
        assert_eq!(d.explanation, None);
    }

    #[test]
    fn unknown_refusal_category_retains_payload() {
        let c: RefusalCategory = serde_json::from_str(r#""some_future_cat""#).unwrap();
        assert_eq!(c, RefusalCategory::Unknown("some_future_cat".into()));
    }

    #[test]
    fn container_decodes() {
        let j = r#"{"id":"container_abc","expires_at":"2026-07-22T00:00:00Z"}"#;
        let c: Container = serde_json::from_str(j).unwrap();
        assert_eq!(c.id, "container_abc");
        assert_eq!(c.expires_at, "2026-07-22T00:00:00Z");
    }

    #[test]
    fn output_tokens_details_decodes() {
        let d: OutputTokensDetails = serde_json::from_str(r#"{"thinking_tokens":7}"#).unwrap();
        assert_eq!(d.thinking_tokens, 7);
    }

    #[test]
    fn server_tool_use_decodes() {
        let s: ServerToolUse =
            serde_json::from_str(r#"{"web_search_requests":2,"web_fetch_requests":3}"#).unwrap();
        assert_eq!(s.web_search_requests, 2);
        assert_eq!(s.web_fetch_requests, 3);
    }

    #[test]
    fn usage_service_tier_variants_and_unknown_decode() {
        let t: UsageServiceTier = serde_json::from_str(r#""priority""#).unwrap();
        assert_eq!(t, UsageServiceTier::Priority);
        let u: UsageServiceTier = serde_json::from_str(r#""future_tier""#).unwrap();
        assert_eq!(u, UsageServiceTier::Unknown("future_tier".into()));
    }

    #[test]
    fn usage_decodes_new_diagnostic_fields() {
        let j = r#"{
            "input_tokens": 10,
            "output_tokens": 20,
            "output_tokens_details": {"thinking_tokens": 8},
            "server_tool_use": {"web_search_requests": 1, "web_fetch_requests": 0},
            "service_tier": "priority",
            "inference_geo": "us"
        }"#;
        let u: Usage = serde_json::from_str(j).unwrap();
        assert_eq!(
            u.output_tokens_details,
            Some(OutputTokensDetails { thinking_tokens: 8 })
        );
        assert_eq!(u.server_tool_use.map(|s| s.web_search_requests), Some(1));
        assert_eq!(u.service_tier, Some(UsageServiceTier::Priority));
        assert_eq!(u.inference_geo.as_deref(), Some("us"));
    }

    #[test]
    fn usage_new_fields_default_to_none_when_absent() {
        let u: Usage = serde_json::from_str(r#"{"input_tokens":1,"output_tokens":2}"#).unwrap();
        assert_eq!(u.output_tokens_details, None);
        assert_eq!(u.server_tool_use, None);
        assert_eq!(u.service_tier, None);
        assert_eq!(u.inference_geo, None);
    }

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
