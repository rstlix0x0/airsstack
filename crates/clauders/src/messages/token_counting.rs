//! Token-counting response type and request projection for
//! `POST /v1/messages/count_tokens`.
//!
//! Exists as its own module so the token-counting surface is only compiled
//! when the `messages-token-counting` feature is enabled.
//!
//! Responsibilities:
//! - Define [`TokenCount`], the response body returned by the endpoint.
//! - Define `CountTokensBody`, a crate-internal serialization projection that
//!   emits only the fields the endpoint accepts. The `count_tokens` method
//!   on [`crate::messages::MessagesResource`] serializes this struct rather
//!   than the full [`crate::messages::MessageRequest`], which carries fields
//!   (`max_tokens`, `temperature`, etc.) the endpoint rejects.
//!
//! Not responsible for:
//! - HTTP dispatch — that lives in `resource.rs`.
//! - Response error decoding — that is `crate::wire_helpers`.

use crate::messages::request::{InputMessage, MessageRequest};
use crate::types::{ModelId, SystemPrompt};

/// Token count returned by `POST /v1/messages/count_tokens`.
///
/// # Examples
///
/// ```
/// use clauders::messages::token_counting::TokenCount;
///
/// let tc: TokenCount = serde_json::from_str(r#"{"input_tokens":42}"#).unwrap();
/// assert_eq!(tc.input_tokens, 42);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct TokenCount {
    /// Number of input tokens the request would consume.
    pub input_tokens: u32,
}

/// Serialization projection for `POST /v1/messages/count_tokens`.
///
/// The count-tokens endpoint accepts only a strict subset of the fields in a
/// full messages request. This struct borrows from a [`MessageRequest`] and
/// serializes only the accepted fields, avoiding a rejection by the API.
///
/// The fields omitted (relative to `MessageRequest`) are:
/// `max_tokens`, `temperature`, `top_p`, `top_k`, `stop_sequences`,
/// `metadata`, and `stream`.
#[derive(serde::Serialize)]
pub(crate) struct CountTokensBody<'a> {
    /// Model to estimate token usage for.
    pub(crate) model: &'a ModelId,
    /// Conversation history to count.
    pub(crate) messages: &'a [InputMessage],
    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) system: Option<&'a SystemPrompt>,
    /// Tool definitions available to the model.
    ///
    /// Only serialized when the `messages-tools` feature is enabled.
    #[cfg(feature = "messages-tools")]
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub(crate) tools: &'a [crate::messages::tools::Tool],
    /// Tool-choice policy.
    ///
    /// Only serialized when the `messages-tools` feature is enabled.
    #[cfg(feature = "messages-tools")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_choice: Option<&'a crate::messages::tools::ToolChoice>,
}

impl<'a> CountTokensBody<'a> {
    /// Build a projection from a [`MessageRequest`] reference.
    ///
    /// Only the fields accepted by the count-tokens endpoint are included;
    /// rejected fields (`max_tokens`, `temperature`, etc.) are silently
    /// omitted.
    #[must_use]
    pub(crate) fn from_request(req: &'a MessageRequest) -> Self {
        Self {
            model: &req.model,
            messages: &req.messages,
            system: req.system.as_ref(),
            #[cfg(feature = "messages-tools")]
            tools: &req.tools,
            #[cfg(feature = "messages-tools")]
            tool_choice: req.tool_choice.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use crate::messages::MessageRequest;
    use crate::types::{MaxTokens, ModelId, Temperature};

    #[test]
    fn token_count_deserializes_from_api_response() {
        let tc: TokenCount = serde_json::from_str(r#"{"input_tokens":99}"#).unwrap();
        assert_eq!(tc.input_tokens, 99);
    }

    #[test]
    fn count_tokens_body_includes_model_and_messages() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(256).unwrap())
            .add_user_text("Hello")
            .build();

        let body = CountTokensBody::from_request(&req);
        let json: serde_json::Value = serde_json::to_value(&body).unwrap();

        assert_eq!(json["model"], "claude-sonnet-4-5");
        assert!(json.get("messages").is_some());
    }

    #[test]
    fn count_tokens_body_omits_max_tokens() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(1024).unwrap())
            .add_user_text("Hello")
            .build();

        let body = CountTokensBody::from_request(&req);
        let json: serde_json::Value = serde_json::to_value(&body).unwrap();

        assert!(
            json.get("max_tokens").is_none(),
            "max_tokens must be absent from the count-tokens body"
        );
    }

    #[test]
    fn count_tokens_body_omits_temperature_and_stream() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .temperature(Temperature::new(0.5).unwrap())
            .add_user_text("Hi")
            .build();

        let body = CountTokensBody::from_request(&req);
        let json: serde_json::Value = serde_json::to_value(&body).unwrap();

        assert!(
            json.get("temperature").is_none(),
            "temperature must be absent"
        );
        assert!(json.get("stream").is_none(), "stream must be absent");
        assert!(
            json.get("stop_sequences").is_none(),
            "stop_sequences must be absent"
        );
    }

    #[test]
    fn count_tokens_body_includes_system_when_present() {
        use crate::types::SystemPrompt;

        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .system(SystemPrompt::text("You are terse."))
            .add_user_text("Hi")
            .build();

        let body = CountTokensBody::from_request(&req);
        let json: serde_json::Value = serde_json::to_value(&body).unwrap();

        assert!(
            json.get("system").is_some(),
            "system must be present when set"
        );
    }

    #[test]
    fn count_tokens_body_omits_system_when_absent() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .add_user_text("Hi")
            .build();

        let body = CountTokensBody::from_request(&req);
        let json: serde_json::Value = serde_json::to_value(&body).unwrap();

        assert!(
            json.get("system").is_none(),
            "system must be absent when not set"
        );
    }
}
