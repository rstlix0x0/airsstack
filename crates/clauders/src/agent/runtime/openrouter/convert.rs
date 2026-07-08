//! Pure impedance mapping between the OpenRouter chat-completions wire surface
//! and the agent frame surface: assistant text extraction, usage and
//! finish-reason conversion, and error folding. No I/O — the unit-test seam of
//! the OpenRouter runtime.

use openrouter_rs::chat::response::{FinishReason, ResponseMessage};
use openrouter_rs::chat::usage::Usage as OrUsage;
use openrouter_rs::error::Error as OrError;

use crate::agent::error::AgentError;
use crate::agent::message::Usage as AgentUsage;

/// The assistant text of a response message, or `""` for a tool-only turn
/// (OpenRouter sends `content: null` when the model emits only tool calls).
pub(super) fn content_text(message: &ResponseMessage) -> String {
    message.content.clone().unwrap_or_default()
}

/// The agent-frame usage subset (input/output tokens) from an OpenRouter usage.
pub(super) fn usage(u: &OrUsage) -> AgentUsage {
    AgentUsage {
        input_tokens: u64::from(u.prompt_tokens),
        output_tokens: u64::from(u.completion_tokens),
        ..AgentUsage::default()
    }
}

/// The result-frame `stop_reason` string for a finish reason, aligned with the
/// vocabulary the `api` runtime already emits.
pub(super) const fn finish_reason_wire(reason: FinishReason) -> &'static str {
    match reason {
        FinishReason::Stop => "end_turn",
        FinishReason::Length => "max_tokens",
        FinishReason::ToolCalls => "tool_use",
        FinishReason::ContentFilter => "refusal",
        FinishReason::Error => "error",
        FinishReason::Unknown => "unknown",
    }
}

/// Fold an OpenRouter client error into the agent error surface.
pub(super) fn map_or_error(error: OrError) -> AgentError {
    match error {
        OrError::Transport(_) => AgentError::TransportClosed,
        OrError::Serde { source, .. } => AgentError::Decode(source.to_string()),
        other => AgentError::Protocol {
            detail: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{content_text, finish_reason_wire, map_or_error, usage};
    use openrouter_rs::chat::response::{FinishReason, ResponseMessage};
    use openrouter_rs::chat::usage::Usage as OrUsage;

    #[test]
    fn content_text_returns_text_when_present() {
        let m: ResponseMessage =
            serde_json::from_value(serde_json::json!({ "role": "assistant", "content": "hi" }))
                .expect("message");
        assert_eq!(content_text(&m), "hi");
    }

    #[test]
    fn content_text_is_empty_when_absent() {
        let m: ResponseMessage =
            serde_json::from_value(serde_json::json!({ "role": "assistant", "content": null }))
                .expect("message");
        assert_eq!(content_text(&m), "");
    }

    #[test]
    fn usage_maps_prompt_and_completion_tokens() {
        let u: OrUsage = serde_json::from_value(serde_json::json!({
            "prompt_tokens": 25, "completion_tokens": 5, "total_tokens": 30
        }))
        .expect("usage");
        let agent = usage(&u);
        assert_eq!(agent.input_tokens, 25);
        assert_eq!(agent.output_tokens, 5);
    }

    #[test]
    fn finish_reason_wire_names_match_the_agent_vocabulary() {
        assert_eq!(finish_reason_wire(FinishReason::Stop), "end_turn");
        assert_eq!(finish_reason_wire(FinishReason::Length), "max_tokens");
        assert_eq!(finish_reason_wire(FinishReason::ToolCalls), "tool_use");
        assert_eq!(finish_reason_wire(FinishReason::ContentFilter), "refusal");
        assert_eq!(finish_reason_wire(FinishReason::Error), "error");
        assert_eq!(finish_reason_wire(FinishReason::Unknown), "unknown");
    }

    #[test]
    fn transport_errors_fold_to_transport_closed() {
        use crate::agent::error::AgentError;
        let err = openrouter_rs::error::Error::InvalidRequest("bad url".into());
        assert!(matches!(map_or_error(err), AgentError::Protocol { .. }));
    }
}
