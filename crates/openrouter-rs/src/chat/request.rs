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
use crate::chat::cache_control::CacheControl;
use crate::chat::message::Message;
use crate::chat::provider::ProviderPreferences;
use crate::chat::response_format::ResponseFormat;
use crate::chat::tool::{Tool, ToolChoice};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) provider: Option<ProviderPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) models: Option<Vec<ModelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cache_control: Option<CacheControl>,
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
    use crate::chat::tool::{FunctionDef, Tool, ToolChoice};
    use crate::types::FunctionName;
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
    fn provider_field_serializes_under_provider_key() {
        use crate::chat::provider::{FallbackPolicy, ProviderPreferences, ProviderSort};
        let prefs = ProviderPreferences::builder()
            .sort(ProviderSort::Price)
            .allow_fallbacks(FallbackPolicy::Allow)
            .build();
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .provider(prefs)
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["provider"]["sort"], json!("price"));
        assert_eq!(v["provider"]["allow_fallbacks"], json!(true));
    }

    #[test]
    fn models_field_serializes_under_models_key() {
        let fallbacks = vec![
            ModelId::custom("anthropic/claude-3-haiku").unwrap(),
            ModelId::custom("openai/gpt-4o-mini").unwrap(),
        ];
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .models(fallbacks)
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(
            v["models"],
            json!(["anthropic/claude-3-haiku", "openai/gpt-4o-mini"])
        );
    }

    #[test]
    fn provider_and_models_omitted_when_unset() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert!(v.get("provider").is_none(), "provider must be absent");
        assert!(v.get("models").is_none(), "models must be absent");
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
    fn top_level_cache_control_serializes_under_cache_control_key() {
        use crate::chat::cache_control::CacheControl;
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .cache_control(CacheControl::ephemeral())
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["cache_control"], json!({ "type": "ephemeral" }));
    }

    #[test]
    fn cache_control_omitted_when_unset() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert!(
            v.get("cache_control").is_none(),
            "cache_control must be absent"
        );
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

    #[test]
    fn tools_and_tool_choice_serialize_when_set() {
        let tool = Tool::function(FunctionDef::new(FunctionName::new("search").unwrap()));
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .tools(vec![tool])
            .tool_choice(ToolChoice::Auto)
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["tool_choice"], json!("auto"));
        assert!(v["tools"].is_array());
        assert_eq!(v["tools"][0]["type"], json!("function"));
        assert_eq!(v["tools"][0]["function"]["name"], json!("search"));
    }

    #[test]
    fn tools_and_tool_choice_omitted_when_not_set() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert!(v.get("tools").is_none(), "tools must be absent");
        assert!(v.get("tool_choice").is_none(), "tool_choice must be absent");
    }

    #[test]
    fn tool_choice_required_serializes_correctly() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .tool_choice(ToolChoice::Required)
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["tool_choice"], json!("required"));
    }

    #[test]
    fn response_format_json_object_serializes() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .response_format(ResponseFormat::JsonObject)
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["response_format"], json!({ "type": "json_object" }));
    }

    #[test]
    fn response_format_omitted_when_not_set() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert!(
            v.get("response_format").is_none(),
            "response_format must be absent"
        );
    }

    #[test]
    fn response_format_json_schema_serializes_full_shape() {
        use crate::chat::response_format::{JsonSchemaConfig, SchemaStrictness};
        use crate::types::SchemaName;
        let schema = json!({ "type": "object", "properties": { "city": { "type": "string" } }, "required": ["city"] });
        let mut cfg = JsonSchemaConfig::new(SchemaName::new("weather").unwrap(), schema);
        cfg.strict = Some(SchemaStrictness::Strict);
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .response_format(ResponseFormat::JsonSchema(cfg))
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["response_format"]["type"], json!("json_schema"));
        assert_eq!(
            v["response_format"]["json_schema"]["name"],
            json!("weather")
        );
        assert_eq!(v["response_format"]["json_schema"]["strict"], json!(true));
    }
}
