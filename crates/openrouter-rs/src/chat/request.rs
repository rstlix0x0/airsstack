//! The chat-completion request payload sent to `POST /chat/completions`.
//!
//! Serialize-only and constructed solely through [`ChatRequest::builder`]; the
//! type-state builder guarantees the required `model` and `messages` are set
//! before a request can be built. Optional parameters are omitted from the wire
//! when unset.
//!
//! Responsibilities:
//! - [`ChatRequest`] — the request body and its `builder()` entry point.

use serde::Serialize;

use crate::chat::builder::{ChatRequestBuilder, Missing};
use crate::chat::message::Message;
use crate::types::{
    FrequencyPenalty, MaxTokens, ModelId, PresencePenalty, RepetitionPenalty, Seed, StopSequences,
    Temperature, TopK, TopP,
};

/// A chat-completion request body.
///
/// Build one through [`ChatRequest::builder`]; required fields (`model`,
/// `messages`) are enforced at compile time by the builder's type state.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::{ChatRequest, Message};
/// use openrouter_rs::types::ModelId;
///
/// let req = ChatRequest::builder()
///     .model(ModelId::custom("openai/gpt-4o").unwrap())
///     .messages(vec![Message::user("hi")])
///     .build();
/// assert_eq!(
///     serde_json::to_value(&req).unwrap(),
///     serde_json::json!({
///         "model": "openai/gpt-4o",
///         "messages": [{ "role": "user", "content": "hi" }]
///     }),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChatRequest {
    pub(crate) model: ModelId,
    pub(crate) messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_tokens: Option<MaxTokens>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<Temperature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) top_p: Option<TopP>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) top_k: Option<TopK>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) seed: Option<Seed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) frequency_penalty: Option<FrequencyPenalty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) presence_penalty: Option<PresencePenalty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repetition_penalty: Option<RepetitionPenalty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stop: Option<StopSequences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user: Option<String>,
    /// Whether to request a streamed response. Managed by the resource layer;
    /// callers do not set this directly.
    #[doc(hidden)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) stream: bool,
}

impl ChatRequest {
    /// Start building a request. Required fields must be set before `build()`.
    #[must_use]
    pub fn builder() -> ChatRequestBuilder<Missing, Missing> {
        ChatRequestBuilder::new()
    }

    /// The target model.
    #[must_use]
    pub const fn model(&self) -> &ModelId {
        &self.model
    }

    /// The conversation messages.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use serde_json::json;

    fn model() -> ModelId {
        ModelId::custom("openai/gpt-4o").unwrap()
    }

    #[test]
    fn minimal_request_serializes_required_only() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        assert_eq!(
            serde_json::to_value(&req).unwrap(),
            json!({ "model": "openai/gpt-4o", "messages": [{ "role": "user", "content": "hi" }] }),
        );
    }

    #[test]
    fn unset_optionals_are_omitted() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        let v = serde_json::to_value(&req).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(obj.len(), 2, "only model + messages should serialize");
    }

    #[test]
    fn stream_flag_omitted_by_default_and_emitted_when_set() {
        let mut req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        // Default build does not request streaming.
        assert!(serde_json::to_value(&req).unwrap().get("stream").is_none());

        // The resource layer flips this before a streaming send.
        req.stream = true;
        assert_eq!(serde_json::to_value(&req).unwrap()["stream"], json!(true));
    }

    #[test]
    fn accessors_expose_required_fields() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi"), Message::assistant("yo")])
            .build();
        assert_eq!(req.model().as_str(), "openai/gpt-4o");
        assert_eq!(req.messages().len(), 2);
    }
}
