//! Top-level message frames streamed from the binary.

use serde::{Deserialize, Serialize};

use crate::agent::content::ContentBlock;
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
}

/// Assistant message payload.
///
/// These frames are inbound only — the binary emits them and the SDK reads
/// them. The `content` field deserializes from the wire's nested `message`
/// object via the `content_from_message` adapter; the `Serialize` impl is not its
/// inverse (it would emit `content` as a bare array), so do not rely on a
/// serialize/deserialize round-trip for this type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// Content blocks in this assistant turn.
    ///
    /// The binary nests these under a `message` object on the wire:
    /// `{"type":"assistant","message":{"content":[...]}}`. The
    /// `content_from_message` adapter lifts them to this flat field.
    #[serde(rename = "message", deserialize_with = "content_from_message")]
    pub content: Vec<ContentBlock>,
    /// Parent tool-use id when this turn answers a tool call.
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
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

/// Terminal result frame for a turn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultMessage {
    /// Final result text.
    #[serde(default)]
    pub result: String,
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

/// Token usage counters reported on a result frame.
///
/// Defined locally rather than reusing `messages::Usage` because the `agent`
/// feature does not enable the `messages` feature; the fields are a tolerant
/// subset and unknown fields are ignored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens consumed.
    #[serde(default)]
    pub input_tokens: u64,
    /// Output tokens produced.
    #[serde(default)]
    pub output_tokens: u64,
}

/// Pull the `content` array out of the binary's nested `message` object.
///
/// The binary wraps the assistant payload as `{"message":{"content":[...]}}`;
/// this adapter lifts `content` to the flat `AssistantMessage.content` field.
fn content_from_message<'de, D>(deserializer: D) -> Result<Vec<ContentBlock>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(default)]
        content: Vec<ContentBlock>,
    }
    let wrapper = Wrapper::deserialize(deserializer)?;
    Ok(wrapper.content)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::Message;

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
