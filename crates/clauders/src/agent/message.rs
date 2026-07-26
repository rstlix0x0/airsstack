//! Top-level message frames streamed from the binary.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::agent::content::ContentBlock;
use crate::agent::model_usage::ModelUsage;
use crate::agent::types::SessionId;

/// A message frame emitted by the binary on its stdout stream.
///
/// Exhaustive enum, internally tagged by the frame's `type` field. The
/// compiler forces consumers to handle every kind so no message is silently
/// dropped. Unknown fields within a variant are tolerated (forward-compat).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// An assistant turn (model output).
    Assistant(AssistantMessage),
    /// A user turn echoed back by the binary.
    User(UserMessage),
    /// A system/control informational frame.
    System(SystemMessage),
    /// The terminal result frame for a turn.
    Result(ResultMessage),
    /// A fine-grained streaming delta event.
    StreamEvent(StreamEvent),
    /// Any frame whose `type` is not one of the above, captured verbatim.
    ///
    /// Keeps the stdout stream forward-compatible: unrecognized frames — such
    /// as the hook-lifecycle frames emitted under `include_hook_events` —
    /// surface here instead of failing the turn. This variant is never
    /// produced by the tagged deserializer (`#[serde(skip)]`); the protocol
    /// codec constructs it for lines that match no known frame. It is
    /// inbound-only and must not be serialized.
    #[serde(skip)]
    Other(serde_json::Value),
}

/// Assistant message payload.
///
/// Inbound only. `content` is lifted flat from the wire's nested `message`
/// object; the message's metadata (`id`, `model`, `role`, stop fields,
/// `usage`) and the frame's identifiers are lifted alongside it. Opaque
/// message-level fields (`stop_details`, `diagnostics`, `context_management`,
/// the nested `type`) are preserved in [`AssistantMessage::extra`]. The
/// `Serialize` impl is not the inverse of deserialize (it emits `content` as
/// a bare array); do not rely on a round-trip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "AssistantWire")]
pub struct AssistantMessage {
    /// Content blocks in this assistant turn.
    pub content: Vec<ContentBlock>,
    /// Parent tool-use id when this turn answers a tool call.
    pub parent_tool_use_id: Option<String>,
    /// Anthropic message id, when reported.
    pub id: Option<String>,
    /// Model that produced the turn, when reported.
    pub model: Option<String>,
    /// Message role (`assistant`), when reported.
    pub role: Option<String>,
    /// Why the model stopped, when reported.
    pub stop_reason: Option<String>,
    /// Stop sequence that ended the turn, when reported.
    pub stop_sequence: Option<String>,
    /// Per-turn token usage, when reported.
    pub usage: Option<Usage>,
    /// Frame uuid, when reported.
    pub uuid: Option<String>,
    /// Session this turn belongs to, when reported.
    pub session_id: Option<SessionId>,
    /// API request id for the turn, when reported.
    pub request_id: Option<String>,
    /// Frame emission timestamp, when reported.
    pub timestamp: Option<String>,
    /// Whether this is an injected/meta turn (absent on normal turns).
    pub is_meta: bool,
    /// Opaque message-level tail (`stop_details`, `diagnostics`,
    /// `context_management`, nested `type`, …).
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub extra: serde_json::Value,
}

/// Wire shape of an assistant frame (private; converted to
/// [`AssistantMessage`]). The frame nests the model message under `message`.
#[derive(Deserialize)]
struct AssistantWire {
    message: AssistantInner,
    #[serde(default)]
    parent_tool_use_id: Option<String>,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    session_id: Option<SessionId>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    is_meta: bool,
}

/// Wire shape of the nested `message` object. Its unmatched keys (the opaque
/// message-level tail) flow into `extra`.
#[derive(Deserialize)]
struct AssistantInner {
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    stop_sequence: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(flatten)]
    extra: serde_json::Value,
}

impl From<AssistantWire> for AssistantMessage {
    fn from(w: AssistantWire) -> Self {
        let m = w.message;
        Self {
            content: m.content,
            parent_tool_use_id: w.parent_tool_use_id,
            id: m.id,
            model: m.model,
            role: m.role,
            stop_reason: m.stop_reason,
            stop_sequence: m.stop_sequence,
            usage: m.usage,
            uuid: w.uuid,
            session_id: w.session_id,
            request_id: w.request_id,
            timestamp: w.timestamp,
            is_meta: w.is_meta,
            extra: m.extra,
        }
    }
}

/// User message payload (echoed by the binary).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "serde_json::Value does not implement Eq; cannot derive it for this struct"
)]
pub struct UserMessage {
    /// Raw user message body as forwarded by the binary.
    #[serde(default)]
    pub message: serde_json::Value,
    /// Parent tool-use id when applicable.
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
}

/// System/control informational frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "serde_json::Value does not implement Eq; cannot derive it for this struct"
)]
pub struct SystemMessage {
    /// Frame subtype (e.g. `init`).
    #[serde(default)]
    pub subtype: Option<String>,
    /// Raw frame body (tolerant — fields vary by subtype).
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Why a turn ended, as reported on the terminal result frame.
///
/// The success case and the four error cases are the closed set the binary
/// ships today; an unrecognized value keeps its wire name rather than being
/// discarded, so a caller can log or match on it.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultSubtype {
    /// The turn completed.
    Success,
    /// The turn failed while executing.
    ErrorDuringExecution,
    /// The turn hit its configured turn limit.
    ErrorMaxTurns,
    /// The turn hit its configured USD budget.
    ErrorMaxBudgetUsd,
    /// Structured output could not be produced within the retry limit.
    ErrorMaxStructuredOutputRetries,
    /// A subtype this release does not model, retained verbatim.
    ///
    /// Deserialize-only: serializing it is an error rather than emitting a
    /// value this release cannot interpret.
    #[serde(untagged, skip_serializing)]
    Unknown(String),
}

/// Terminal result frame for a turn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultMessage {
    /// Why the turn ended.
    pub subtype: ResultSubtype,
    /// Diagnostics attached to an error result.
    ///
    /// `SDKResultError` declares this required; `SDKResultSuccess` has no
    /// such field, so it defaults to empty rather than failing a success
    /// frame.
    #[serde(default)]
    pub errors: Vec<String>,
    /// Final result text.
    #[serde(default)]
    pub result: String,
    /// Parsed structured output when a schema was requested; `None` otherwise.
    #[serde(default)]
    pub structured_output: Option<serde_json::Value>,
    /// Whether the turn ended in error.
    #[serde(default)]
    pub is_error: bool,
    /// Total cost in USD if reported.
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
    /// Stop reason if reported.
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// Token usage if reported.
    #[serde(default)]
    pub usage: Option<Usage>,
    /// Session this result belongs to.
    pub session_id: SessionId,
    /// Number of turns taken.
    #[serde(default)]
    pub num_turns: u32,
    /// Per-model cost/token breakdown, keyed by model id.
    #[serde(rename = "modelUsage", default)]
    pub model_usage: HashMap<String, ModelUsage>,
    /// Permission denials recorded during the turn. Element shape is not yet
    /// grounded, so this is a tolerant value list rather than a typed element.
    #[serde(default)]
    pub permission_denials: Vec<serde_json::Value>,
    /// Wall-clock duration of the turn in milliseconds, when reported.
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// API-time duration of the turn in milliseconds, when reported.
    #[serde(default)]
    pub duration_api_ms: Option<u64>,
    /// Time to first token in milliseconds, when reported.
    #[serde(default)]
    pub ttft_ms: Option<u64>,
    /// Why the turn terminated (`completed`, …), when reported.
    #[serde(default)]
    pub terminal_reason: Option<String>,
    /// Frame uuid, when reported.
    #[serde(default)]
    pub uuid: Option<String>,
    /// Forward-compatible / lower-value tail (`fast_mode_state`,
    /// `api_error_status`, `ttft_stream_ms`, `time_to_request_ms`, …).
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Fine-grained streaming event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "serde_json::Value does not implement Eq; cannot derive it for this struct"
)]
pub struct StreamEvent {
    /// The raw event payload (opaque — shape varies by event).
    #[serde(default)]
    pub event: serde_json::Value,
}

/// Token usage counters reported on a result or assistant frame.
///
/// A tolerant subset: the stable counters are typed; the evolving remainder
/// (`iterations`, `speed`, and any future field) is preserved in
/// [`Usage::extra`]. Unknown fields are never an error.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "every field is Eq-capable today so clippy suggests deriving Eq; Eq is withheld deliberately to keep the type non-Eq as these frames gain float-valued counters"
)]
pub struct Usage {
    /// Input tokens consumed.
    #[serde(default)]
    pub input_tokens: u64,
    /// Output tokens produced.
    #[serde(default)]
    pub output_tokens: u64,
    /// Input tokens written to the prompt cache on this turn, when reported.
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    /// Input tokens served from the prompt cache on this turn, when reported.
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    /// Nested cache-creation breakdown (`ephemeral_*_input_tokens`), opaque.
    #[serde(default)]
    pub cache_creation: Option<serde_json::Value>,
    /// Nested server-tool usage (`web_search_requests`, …), opaque.
    #[serde(default)]
    pub server_tool_use: Option<serde_json::Value>,
    /// Service tier that served the request, when reported.
    #[serde(default)]
    pub service_tier: Option<String>,
    /// Inference geography, when reported.
    #[serde(default)]
    pub inference_geo: Option<String>,
    /// Forward-compatible extras (`iterations`, `speed`, …).
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::{Message, ResultMessage, ResultSubtype, Usage};
    use crate::agent::types::SessionId;

    #[test]
    fn usage_carries_cache_counters_when_present() {
        let json = r#"{"input_tokens":10,"output_tokens":2,"cache_creation_input_tokens":100,"cache_read_input_tokens":40}"#;
        let u: Usage = serde_json::from_str(json).expect("usage");
        assert_eq!(u.cache_creation_input_tokens, Some(100));
        assert_eq!(u.cache_read_input_tokens, Some(40));
    }

    #[test]
    fn usage_cache_counters_default_to_none() {
        let json = r#"{"input_tokens":10,"output_tokens":2}"#;
        let u: Usage = serde_json::from_str(json).expect("usage");
        assert_eq!(u.cache_creation_input_tokens, None);
        assert_eq!(u.cache_read_input_tokens, None);
    }

    #[test]
    fn usage_carries_extended_counters_and_extra() {
        // captured: claude -p "say hi" --output-format stream-json --verbose
        let json = r#"{"input_tokens":2,"cache_creation_input_tokens":25595,"cache_read_input_tokens":0,
          "output_tokens":6,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},
          "service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":25595,"ephemeral_5m_input_tokens":0},
          "inference_geo":"not_available",
          "iterations":[{"input_tokens":2,"type":"message"}],"speed":"standard"}"#;
        let u: Usage = serde_json::from_str(json).expect("usage");
        assert_eq!(u.service_tier.as_deref(), Some("standard"));
        assert_eq!(u.inference_geo.as_deref(), Some("not_available"));
        assert!(u.cache_creation.is_some());
        assert!(u.server_tool_use.is_some());
        // fields the struct does not type survive in extra
        assert!(u.extra.get("iterations").is_some());
        assert_eq!(
            u.extra.get("speed").and_then(|v| v.as_str()),
            Some("standard")
        );
    }

    #[test]
    fn assistant_message_lifts_metadata_and_preserves_inner_tail() {
        // captured: claude -p "say hi" --output-format stream-json --verbose
        let json = r#"{"type":"assistant","message":{"model":"claude-opus-4-8","id":"msg_1",
          "type":"message","role":"assistant","content":[{"type":"text","text":"Hi."}],
          "stop_reason":null,"stop_sequence":null,"stop_details":null,
          "usage":{"input_tokens":2,"output_tokens":3,"service_tier":"standard"},
          "diagnostics":null,"context_management":null},
          "parent_tool_use_id":null,"session_id":"s1","uuid":"6f2c34e3",
          "timestamp":"2026-07-26T09:34:30.189Z","request_id":"req_1"}"#;
        let msg: Message = serde_json::from_str(json).expect("deserialize");
        let Message::Assistant(a) = msg else {
            panic!("expected Assistant")
        };
        // content still lifted flat
        assert_eq!(a.content.len(), 1);
        // lifted metadata
        assert_eq!(a.id.as_deref(), Some("msg_1"));
        assert_eq!(a.model.as_deref(), Some("claude-opus-4-8"));
        assert_eq!(a.role.as_deref(), Some("assistant"));
        assert_eq!(a.uuid.as_deref(), Some("6f2c34e3"));
        assert_eq!(a.session_id.as_ref().map(SessionId::as_str), Some("s1"));
        assert_eq!(a.request_id.as_deref(), Some("req_1"));
        assert_eq!(a.timestamp.as_deref(), Some("2026-07-26T09:34:30.189Z"));
        assert!(a.usage.is_some());
        assert!(!a.is_meta);
        // opaque message-level fields survive in extra (nested → inner flatten)
        assert!(a.extra.get("stop_details").is_some());
        assert!(a.extra.get("diagnostics").is_some());
        assert!(a.extra.get("context_management").is_some());
        assert_eq!(
            a.extra.get("type").and_then(|v| v.as_str()),
            Some("message")
        );
    }

    #[test]
    fn assistant_message_minimal_frame_defaults_and_empty_extra() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hi"}]},
          "parent_tool_use_id":null,"session_id":"s1"}"#;
        let msg: Message = serde_json::from_str(json).expect("deserialize");
        let Message::Assistant(a) = msg else {
            panic!("expected Assistant")
        };
        assert_eq!(a.content.len(), 1);
        assert!(a.id.is_none());
        assert!(a.usage.is_none());
        assert!(!a.is_meta);
        assert_eq!(a.extra, serde_json::json!({}));
    }

    #[test]
    fn deserializes_assistant_message() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hi"}]},"parent_tool_use_id":null,"session_id":"s1"}"#;
        let msg: Message = serde_json::from_str(json).expect("deserialize");
        match msg {
            Message::Assistant(a) => {
                assert_eq!(a.content.len(), 1);
                assert!(a.parent_tool_use_id.is_none());
            }
            other => panic!("expected Assistant, got {other:?}"),
        }
    }

    #[test]
    fn result_message_surfaces_metadata_and_preserves_tail() {
        // captured: claude -p "say hi" --output-format stream-json --verbose
        let json = r#"{"type":"result","subtype":"success","is_error":false,"result":"Hi.",
          "num_turns":1,"session_id":"s1","total_cost_usd":0.25611,"stop_reason":"end_turn",
          "duration_ms":1971,"duration_api_ms":1884,"ttft_ms":1945,"ttft_stream_ms":1845,
          "time_to_request_ms":85,"terminal_reason":"completed","fast_mode_state":"off",
          "api_error_status":null,"uuid":"24e65d43-c456-4ff3-b069-4a03cc671061",
          "permission_denials":[],
          "modelUsage":{"claude-opus-4-8[1m]":{"inputTokens":2,"outputTokens":6,"costUSD":0.25611,
            "contextWindow":1000000,"maxOutputTokens":64000}}}"#;
        let msg: Message = serde_json::from_str(json).expect("deserialize");
        let Message::Result(r) = msg else {
            panic!("expected Result")
        };
        assert_eq!(r.duration_ms, Some(1971));
        assert_eq!(r.duration_api_ms, Some(1884));
        assert_eq!(r.ttft_ms, Some(1945));
        assert_eq!(r.terminal_reason.as_deref(), Some("completed"));
        assert_eq!(
            r.uuid.as_deref(),
            Some("24e65d43-c456-4ff3-b069-4a03cc671061")
        );
        assert!(r.permission_denials.is_empty());
        let mu = r
            .model_usage
            .get("claude-opus-4-8[1m]")
            .expect("model usage present");
        assert_eq!(mu.output_tokens, 6);
        assert_eq!(mu.context_window, 1_000_000);
        // lower-value / opaque fields survive in extra
        assert_eq!(
            r.extra.get("fast_mode_state").and_then(|v| v.as_str()),
            Some("off")
        );
        assert!(r.extra.get("ttft_stream_ms").is_some());
        assert!(r.extra.get("time_to_request_ms").is_some());
        assert!(r.extra.get("api_error_status").is_some());
    }

    #[test]
    fn deserializes_result_message() {
        let json = r#"{"type":"result","subtype":"success","is_error":false,"result":"done","num_turns":3,"session_id":"s1","total_cost_usd":0.01}"#;
        let msg: Message = serde_json::from_str(json).expect("deserialize");
        match msg {
            Message::Result(r) => {
                assert_eq!(r.result, "done");
                assert!(!r.is_error);
                assert_eq!(r.num_turns, 3);
                assert_eq!(r.session_id.as_str(), "s1");
                assert_eq!(r.total_cost_usd, Some(0.01));
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[test]
    fn result_message_defaults_structured_output_to_none_on_deserialize() {
        let json = serde_json::json!({
            "subtype": "success",
            "result": "hi",
            "session_id": "s1",
            "num_turns": 1
        });
        let msg: ResultMessage = serde_json::from_value(json).expect("deserialize");
        assert!(msg.structured_output.is_none());
    }

    #[test]
    fn result_message_carries_structured_output_when_present() {
        let json = serde_json::json!({
            "subtype": "success",
            "result": "{\"city\":\"Paris\"}",
            "structured_output": { "city": "Paris" },
            "session_id": "s1",
            "num_turns": 1
        });
        let msg: ResultMessage = serde_json::from_value(json).expect("deserialize");
        assert_eq!(
            msg.structured_output,
            Some(serde_json::json!({ "city": "Paris" }))
        );
    }

    #[test]
    fn other_variant_holds_raw_value() {
        let m = Message::Other(serde_json::json!({ "type": "hook_progress" }));
        match m {
            Message::Other(v) => assert_eq!(v["type"], "hook_progress"),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn error_result_carries_subtype_and_diagnostics() {
        // Field set copied from SDKResultError, sdk.d.ts:4230-4248.
        let json = r#"{
            "type":"result","subtype":"error_max_turns","duration_ms":1,
            "duration_api_ms":1,"is_error":true,"num_turns":9,"stop_reason":null,
            "total_cost_usd":0.5,"session_id":"s1","errors":["turn limit reached"]
        }"#;
        let message: Message = serde_json::from_str(json).expect("deserialize");
        let Message::Result(result) = message else {
            panic!("expected Result");
        };
        assert_eq!(result.subtype, ResultSubtype::ErrorMaxTurns);
        assert!(!result.errors.is_empty(), "diagnostics must survive");
        assert_eq!(result.errors[0], "turn limit reached");
    }

    #[test]
    fn success_result_has_no_diagnostics() {
        let json = r#"{
            "type":"result","subtype":"success","duration_ms":1,"duration_api_ms":1,
            "is_error":false,"num_turns":2,"session_id":"s1","result":"done"
        }"#;
        let message: Message = serde_json::from_str(json).expect("deserialize");
        let Message::Result(result) = message else {
            panic!("expected Result");
        };
        assert_eq!(result.subtype, ResultSubtype::Success);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn unmodelled_result_subtype_retains_its_wire_name() {
        let subtype: ResultSubtype =
            serde_json::from_str("\"error_something_new\"").expect("deserialize");
        assert_eq!(
            subtype,
            ResultSubtype::Unknown("error_something_new".to_string())
        );
    }

    #[test]
    fn deserializes_system_and_stream_event() {
        let sys = r#"{"type":"system","subtype":"init","session_id":"s1"}"#;
        assert!(matches!(
            serde_json::from_str::<Message>(sys).expect("system"),
            Message::System(_)
        ));
        let ev = r#"{"type":"stream_event","event":{"foo":1},"session_id":"s1"}"#;
        assert!(matches!(
            serde_json::from_str::<Message>(ev).expect("stream_event"),
            Message::StreamEvent(_)
        ));
    }
}
