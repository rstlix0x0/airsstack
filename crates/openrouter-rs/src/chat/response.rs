//! Decoded non-streaming chat-completion response.
//!
//! Plain `Deserialize` data carriers. Unknown `finish_reason` values decode to
//! [`FinishReason::Unknown`] rather than failing, so a new server value never
//! breaks an existing client.
//!
//! Responsibilities:
//! - [`ChatCompletion`] — the response envelope.
//! - [`Choice`] — one returned completion choice.
//! - [`ResponseMessage`] — the assistant message inside a choice, including
//!   any `tool_calls` the model emitted.
//! - [`FinishReason`] — why generation stopped (unknown-tolerant).
//!
//! Not responsible for HTTP status handling or error-envelope decode — the
//! resource layer maps non-success responses to errors.

use serde::Deserialize;

use crate::chat::message::Role;
use crate::chat::tool_call::ToolCall;
use crate::chat::usage::Usage;

/// A non-streaming chat completion.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::{ChatCompletion, FinishReason};
/// let body = serde_json::json!({
///     "id": "gen-1", "object": "chat.completion", "created": 1, "model": "openai/gpt-4o",
///     "choices": [{
///         "index": 0,
///         "message": { "role": "assistant", "content": "4" },
///         "finish_reason": "stop"
///     }]
/// });
/// let c: ChatCompletion = serde_json::from_value(body).unwrap();
/// assert_eq!(c.choices[0].finish_reason, Some(FinishReason::Stop));
/// assert_eq!(c.choices[0].message.content.as_deref(), Some("4"));
/// ```
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ChatCompletion {
    /// Generation id (`gen-…`).
    pub id: String,
    /// Object discriminator; `"chat.completion"` for this endpoint.
    pub object: String,
    /// Unix creation timestamp (seconds).
    pub created: u64,
    /// The model that produced the completion, as echoed by the server.
    pub model: String,
    /// The returned choices (one unless `n` was set).
    pub choices: Vec<Choice>,
    /// Token usage, when reported.
    #[serde(default)]
    pub usage: Option<Usage>,
    /// Provider system fingerprint, when reported.
    #[serde(default)]
    pub system_fingerprint: Option<String>,
}

/// One completion choice.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Choice {
    /// Position of this choice in `choices`.
    pub index: u32,
    /// The assistant message for this choice.
    pub message: ResponseMessage,
    /// Why generation stopped, when reported.
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    /// Log-probability payload, passed through untyped when present.
    #[serde(default)]
    pub logprobs: Option<serde_json::Value>,
}

/// The assistant message inside a [`Choice`].
///
/// `content` is optional because a tool-only response carries `null` content.
/// When the model chooses to call tools, `tool_calls` carries the call list
/// and `finish_reason` is [`FinishReason::ToolCalls`].
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ResponseMessage {
    /// Author role (`assistant` for model output).
    pub role: Role,
    /// The generated text, when present.
    #[serde(default)]
    pub content: Option<String>,
    /// Tool calls the model wants to make, when the model chose to use tools.
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Why a completion stopped generating.
///
/// Unknown server values decode to [`FinishReason::Unknown`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural stop / stop sequence hit.
    Stop,
    /// Hit the max-token limit.
    Length,
    /// The model emitted tool calls.
    ToolCalls,
    /// Output was filtered by content moderation.
    ContentFilter,
    /// Generation errored mid-flight.
    Error,
    /// A value this client does not recognize.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "tests unwrap/expect known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use serde_json::json;

    fn sample() -> serde_json::Value {
        json!({
            "id": "gen-abc", "object": "chat.completion", "created": 1_700_000_000_u64,
            "model": "anthropic/claude-sonnet-4-5",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "hello" },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 5, "completion_tokens": 1, "total_tokens": 6 }
        })
    }

    #[test]
    fn decodes_full_completion() {
        let c: ChatCompletion = serde_json::from_value(sample()).unwrap();
        assert_eq!(c.id, "gen-abc");
        assert_eq!(c.object, "chat.completion");
        assert_eq!(c.model, "anthropic/claude-sonnet-4-5");
        assert_eq!(c.choices.len(), 1);
        assert_eq!(c.choices[0].index, 0);
        assert_eq!(c.choices[0].message.role, Role::Assistant);
        assert_eq!(c.choices[0].message.content.as_deref(), Some("hello"));
        assert_eq!(c.choices[0].finish_reason, Some(FinishReason::Stop));
        assert_eq!(c.usage.unwrap().total_tokens, 6);
        assert!(c.system_fingerprint.is_none());
    }

    #[test]
    fn unknown_finish_reason_decodes_to_unknown() {
        let r: FinishReason = serde_json::from_value(json!("something_new")).unwrap();
        assert_eq!(r, FinishReason::Unknown);
    }

    #[test]
    fn each_known_finish_reason_decodes() {
        for (wire, want) in [
            ("stop", FinishReason::Stop),
            ("length", FinishReason::Length),
            ("tool_calls", FinishReason::ToolCalls),
            ("content_filter", FinishReason::ContentFilter),
            ("error", FinishReason::Error),
        ] {
            let r: FinishReason = serde_json::from_value(json!(wire)).unwrap();
            assert_eq!(r, want);
        }
    }

    #[test]
    fn null_content_message_decodes() {
        let m: ResponseMessage =
            serde_json::from_value(json!({ "role": "assistant", "content": null })).unwrap();
        assert_eq!(m.content, None);
    }

    #[test]
    fn missing_optional_fields_default_to_none() {
        let body = json!({
            "id": "g", "object": "chat.completion", "created": 1, "model": "x/y",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "ok" } }]
        });
        let c: ChatCompletion = serde_json::from_value(body).unwrap();
        assert!(c.usage.is_none());
        assert!(c.choices[0].finish_reason.is_none());
        assert!(c.choices[0].logprobs.is_none());
    }

    #[test]
    fn response_message_decodes_tool_calls() {
        let body = json!({
            "id": "gen-tc", "object": "chat.completion", "created": 1, "model": "openai/gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "search_books",
                            "arguments": "{\"q\":\"rust programming\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let c: ChatCompletion = serde_json::from_value(body).unwrap();
        let msg = &c.choices[0].message;
        assert!(msg.content.is_none());
        let tool_calls = msg.tool_calls.as_ref().expect("tool_calls present");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id.as_str(), "call_abc123");
        assert_eq!(tool_calls[0].function.name, "search_books");
        assert_eq!(
            tool_calls[0].function.arguments,
            r#"{"q":"rust programming"}"#
        );
        assert_eq!(c.choices[0].finish_reason, Some(FinishReason::ToolCalls));
    }

    #[test]
    fn response_message_tool_calls_absent_when_no_tools() {
        let c: ChatCompletion = serde_json::from_value(sample()).unwrap();
        assert!(c.choices[0].message.tool_calls.is_none());
    }

    #[test]
    fn response_message_decodes_multiple_tool_calls() {
        let body = json!({
            "id": "gen-multi", "object": "chat.completion", "created": 1, "model": "openai/gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "type": "function",
                            "function": { "name": "fn_a", "arguments": "{}" }
                        },
                        {
                            "id": "call_2",
                            "type": "function",
                            "function": { "name": "fn_b", "arguments": "{\"x\":1}" }
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let c: ChatCompletion = serde_json::from_value(body).unwrap();
        let tool_calls = c.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0].id.as_str(), "call_1");
        assert_eq!(tool_calls[1].function.name, "fn_b");
    }
}
