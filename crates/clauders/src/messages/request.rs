//! Request types for the Messages API.
//!
//! Exists as its own module so the request builder and its type-state
//! markers are scoped separately from response decoding and transport wiring.
//!
//! Responsibilities:
//! - Define [`Role`], [`InputMessage`], [`MessageContent`], [`Metadata`],
//!   and [`MessageRequest`] — the wire-format request struct.
//! - Provide a type-state [`MessageRequestBuilder`] that enforces `model`
//!   and `max_tokens` are set before `build()` compiles.
//!
//! Not responsible for:
//! - Sending the request — that is `resource.rs`.
//! - Response decoding — that is `response.rs`.

use std::marker::PhantomData;

use crate::messages::content::ContentBlockParam;
use crate::types::{
    MaxTokens, ModelId, StopSequence, SystemPrompt, Temperature, TopK, TopP, UserId,
};

// ── Sealed type-state markers ────────────────────────────────────────────────
// These are local to this module. The client builder uses identically-named
// markers that live in `crate::builder` under its own sealed trait — the names
// do not conflict because each set is scoped behind its own `mod sealed` block.

mod sealed {
    pub trait BuilderModelState {}
    pub trait BuilderMaxTokensState {}
}

/// Marker: a required builder field has not been supplied yet.
pub struct Missing;
/// Marker: a required builder field has been supplied.
pub struct Present;

impl sealed::BuilderModelState for Missing {}
impl sealed::BuilderModelState for Present {}
impl sealed::BuilderMaxTokensState for Missing {}
impl sealed::BuilderMaxTokensState for Present {}

// ── Role ─────────────────────────────────────────────────────────────────────

/// Message author role.
///
/// # Examples
///
/// ```
/// use clauders::messages::Role;
/// let j = serde_json::to_string(&Role::User).unwrap();
/// assert_eq!(j, "\"user\"");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A message from the end user.
    User,
    /// A message from the assistant.
    Assistant,
    /// A mid-conversation system message (GA on current models).
    System,
}

// ── Request-side value types ────────────────────────────────────────────────

/// Request-side service-tier selector.
///
/// Distinct from the response-side [`crate::messages::UsageServiceTier`], which
/// reports the tier that actually served the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestServiceTier {
    /// Let the server choose the tier.
    Auto,
    /// Use only the standard tier; never fall back.
    StandardOnly,
}

/// Geographic region selector for inference (`inference_geo` request param).
///
/// Open-valued: any region string the workspace accepts. When unset, the
/// workspace's `default_inference_geo` is used.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct InferenceGeo(String);

impl InferenceGeo {
    /// Wrap a region identifier.
    #[must_use]
    pub fn new(region: impl Into<String>) -> Self {
        Self(region.into())
    }

    /// Borrow the region identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Identifier of a code-execution container to reuse (`container` request param).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct ContainerId(String);

impl ContainerId {
    /// Wrap a container identifier.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the container identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── MessageContent ────────────────────────────────────────────────────────────

/// Content of an input message: either a plain string or an array of
/// content blocks.
///
/// The untagged representation means serde picks the variant purely from
/// the JSON shape: a JSON string → `Text`, a JSON array → `Blocks`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain text — serialized as a bare JSON string.
    Text(String),
    /// Typed content blocks — serialized as a JSON array.
    Blocks(Vec<ContentBlockParam>),
}

// ── InputMessage ──────────────────────────────────────────────────────────────

/// A single input turn in the conversation history.
///
/// # Examples
///
/// ```
/// use clauders::messages::{InputMessage, MessageContent, Role};
/// let msg = InputMessage {
///     role: Role::User,
///     content: MessageContent::Text("Hello".into()),
/// };
/// let j = serde_json::to_string(&msg).unwrap();
/// assert!(j.contains("\"role\":\"user\""));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InputMessage {
    /// Author of this turn.
    pub role: Role,
    /// Content of this turn.
    pub content: MessageContent,
}

// ── Metadata ──────────────────────────────────────────────────────────────────

/// Optional per-request metadata sent to the API.
///
/// # Examples
///
/// ```
/// use clauders::messages::Metadata;
/// use clauders::types::UserId;
/// let uid = UserId::new("user-42").unwrap();
/// let meta = Metadata { user_id: Some(uid) };
/// let j = serde_json::to_string(&meta).unwrap();
/// assert!(j.contains("user-42"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    /// End-user identifier forwarded in `metadata.user_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<UserId>,
}

// ── MessageRequest ────────────────────────────────────────────────────────────

/// Wire-format request body for `POST /v1/messages`.
///
/// Construct via [`MessageRequest::builder`], which enforces that `model`
/// and `max_tokens` are provided before the request can be built.
///
/// # Examples
///
/// ```
/// use clauders::messages::MessageRequest;
/// use clauders::types::{MaxTokens, ModelId};
///
/// let req = MessageRequest::builder()
///     .model(ModelId::claude_sonnet_5())
///     .max_tokens(MaxTokens::new(1024))
///     .add_user_text("Hello, Claude")
///     .build();
///
/// assert_eq!(req.model.as_str(), "claude-sonnet-5");
/// ```
#[derive(Clone, Debug, serde::Serialize)]
pub struct MessageRequest {
    /// Model to invoke.
    pub model: ModelId,
    /// Maximum tokens to generate.
    pub max_tokens: MaxTokens,
    /// Conversation history.
    pub messages: Vec<InputMessage>,
    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,
    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<Temperature>,
    /// Nucleus sampling probability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<TopP>,
    /// Top-k sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<TopK>,
    /// Stop sequences to halt generation.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<StopSequence>,
    /// Optional per-request metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    /// Tools the model may call during generation.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<crate::messages::tools::Tool>,
    /// Controls which tool, if any, the model must call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<crate::messages::tools::ToolChoice>,
    /// Output-shape constraint applied to the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<crate::messages::structured_outputs::OutputConfig>,
    /// Extended-thinking configuration for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<crate::messages::thinking::ThinkingConfig>,
    /// Service-tier selector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<RequestServiceTier>,
    /// Geographic inference region.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<InferenceGeo>,
    /// Code-execution container to reuse.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerId>,
    /// Top-level cache-control breakpoint; the server auto-places it on the
    /// last cacheable block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<crate::types::CacheControl>,
    /// Whether to stream the response. Managed by the resource layer;
    /// callers should not set this directly.
    #[doc(hidden)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) stream: bool,
}

impl MessageRequest {
    /// Return a builder for constructing a `MessageRequest`.
    ///
    /// The builder enforces that both `model` and `max_tokens` are supplied
    /// before `build()` becomes callable.
    #[must_use]
    pub const fn builder() -> MessageRequestBuilder<Missing, Missing> {
        MessageRequestBuilder::new()
    }
}

// ── MessageRequestBuilder ─────────────────────────────────────────────────────

// All mutable data fields live in one private struct so that type-state
// transitions (`model`, `max_tokens`) move this whole value without
// enumerating individual fields. Adding a new field here requires edits
// only to: this struct definition, its `const fn new()`, the relevant
// setter, and `build()`.
struct MessageRequestFields {
    model: Option<ModelId>,
    max_tokens: Option<MaxTokens>,
    messages: Vec<InputMessage>,
    system: Option<SystemPrompt>,
    temperature: Option<Temperature>,
    top_p: Option<TopP>,
    top_k: Option<TopK>,
    stop_sequences: Vec<StopSequence>,
    metadata: Option<Metadata>,
    tools: Vec<crate::messages::tools::Tool>,
    tool_choice: Option<crate::messages::tools::ToolChoice>,
    output_format: Option<crate::messages::structured_outputs::OutputFormat>,
    effort: Option<crate::types::EffortLevel>,
    thinking: Option<crate::messages::thinking::ThinkingConfig>,
    service_tier: Option<RequestServiceTier>,
    inference_geo: Option<InferenceGeo>,
    container: Option<ContainerId>,
    cache_control: Option<crate::types::CacheControl>,
}

impl MessageRequestFields {
    const fn new() -> Self {
        Self {
            model: None,
            max_tokens: None,
            messages: Vec::new(),
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: Vec::new(),
            metadata: None,
            tools: Vec::new(),
            tool_choice: None,
            output_format: None,
            effort: None,
            thinking: None,
            service_tier: None,
            inference_geo: None,
            container: None,
            cache_control: None,
        }
    }
}

/// Type-state builder for [`MessageRequest`].
///
/// `M` encodes whether `model` has been supplied; `Mt` encodes whether
/// `max_tokens` has been supplied. `build()` is only callable once both
/// are `Present`.
pub struct MessageRequestBuilder<M, Mt>
where
    M: sealed::BuilderModelState,
    Mt: sealed::BuilderMaxTokensState,
{
    fields: MessageRequestFields,
    _m: PhantomData<M>,
    _mt: PhantomData<Mt>,
}

impl MessageRequestBuilder<Missing, Missing> {
    const fn new() -> Self {
        Self {
            fields: MessageRequestFields::new(),
            _m: PhantomData,
            _mt: PhantomData,
        }
    }
}

impl<Mt: sealed::BuilderMaxTokensState> MessageRequestBuilder<Missing, Mt> {
    /// Set the model. Transitions the `model` type-state from `Missing`
    /// to `Present`.
    #[must_use]
    pub fn model(self, model: ModelId) -> MessageRequestBuilder<Present, Mt> {
        let mut fields = self.fields;
        fields.model = Some(model);
        MessageRequestBuilder {
            fields,
            _m: PhantomData,
            _mt: self._mt,
        }
    }
}

impl<M: sealed::BuilderModelState> MessageRequestBuilder<M, Missing> {
    /// Set `max_tokens`. Transitions the `max_tokens` type-state from
    /// `Missing` to `Present`.
    #[must_use]
    pub fn max_tokens(self, max_tokens: MaxTokens) -> MessageRequestBuilder<M, Present> {
        let mut fields = self.fields;
        fields.max_tokens = Some(max_tokens);
        MessageRequestBuilder {
            fields,
            _m: self._m,
            _mt: PhantomData,
        }
    }
}

impl<M: sealed::BuilderModelState, Mt: sealed::BuilderMaxTokensState> MessageRequestBuilder<M, Mt> {
    /// Append a user-role plain-text message.
    #[must_use]
    pub fn add_user_text(mut self, text: impl Into<String>) -> Self {
        self.fields.messages.push(InputMessage {
            role: Role::User,
            content: MessageContent::Text(text.into()),
        });
        self
    }

    /// Append an assistant-role plain-text message.
    #[must_use]
    pub fn add_assistant_text(mut self, text: impl Into<String>) -> Self {
        self.fields.messages.push(InputMessage {
            role: Role::Assistant,
            content: MessageContent::Text(text.into()),
        });
        self
    }

    /// Append a `system`-role text message to the conversation.
    #[must_use]
    pub fn add_system_text(mut self, text: impl Into<String>) -> Self {
        self.fields.messages.push(InputMessage {
            role: Role::System,
            content: MessageContent::Text(text.into()),
        });
        self
    }

    /// Append a message with an explicit role and content.
    #[must_use]
    pub fn add_message(mut self, role: Role, content: MessageContent) -> Self {
        self.fields.messages.push(InputMessage { role, content });
        self
    }

    /// Set a system prompt.
    #[must_use]
    pub fn system(mut self, system: SystemPrompt) -> Self {
        self.fields.system = Some(system);
        self
    }

    /// Set the sampling temperature.
    ///
    /// # Deprecated
    ///
    /// Models released after Claude Opus 4.6 do not support setting
    /// `temperature`. A value of `1.0` is accepted for backwards
    /// compatibility; any other value is rejected with a 400. Prefer
    /// `output_config.effort` to influence generation on current models.
    #[deprecated(
        note = "Models released after Claude Opus 4.6 do not support setting temperature; \
                only 1.0 is accepted, any other value is rejected with a 400. \
                Prefer output_config.effort."
    )]
    #[must_use]
    pub const fn temperature(mut self, temperature: Temperature) -> Self {
        self.fields.temperature = Some(temperature);
        self
    }

    /// Set the top-p nucleus sampling probability.
    ///
    /// # Deprecated
    ///
    /// Models released after Claude Opus 4.6 do not support setting `top_p`.
    /// A value `>= 0.99` is accepted for backwards compatibility; anything
    /// lower is rejected with a 400.
    #[deprecated(
        note = "Models released after Claude Opus 4.6 do not support setting top_p; \
                only values >= 0.99 are accepted, anything lower is rejected with a 400."
    )]
    #[must_use]
    pub const fn top_p(mut self, top_p: TopP) -> Self {
        self.fields.top_p = Some(top_p);
        self
    }

    /// Set the top-k sampling parameter.
    ///
    /// # Deprecated
    ///
    /// Models released after Claude Opus 4.6 do not accept `top_k` at all;
    /// any value is rejected with a 400.
    #[deprecated(note = "Models released after Claude Opus 4.6 do not accept top_k; \
                any value is rejected with a 400.")]
    #[must_use]
    pub const fn top_k(mut self, top_k: TopK) -> Self {
        self.fields.top_k = Some(top_k);
        self
    }

    /// Set stop sequences.
    ///
    /// Accepts any iterable — `Vec`, array literal, or iterator — of
    /// [`StopSequence`] values. The collected sequences replace any previously
    /// set value.
    #[must_use]
    pub fn stop_sequences(mut self, ss: impl IntoIterator<Item = StopSequence>) -> Self {
        self.fields.stop_sequences = ss.into_iter().collect();
        self
    }

    /// Set per-request metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.fields.metadata = Some(metadata);
        self
    }

    /// Set the tools available to the model.
    ///
    /// Accepts any iterable of [`crate::messages::tools::Tool`] values.
    /// The collected tools replace any previously set value.
    #[must_use]
    pub fn tools(mut self, tools: impl IntoIterator<Item = crate::messages::tools::Tool>) -> Self {
        self.fields.tools = tools.into_iter().collect();
        self
    }

    /// Set the tool-choice policy for this request.
    #[must_use]
    pub fn tool_choice(mut self, c: crate::messages::tools::ToolChoice) -> Self {
        self.fields.tool_choice = Some(c);
        self
    }

    /// Set the output configuration.
    ///
    /// This setter names **both** slots the configuration carries, so it
    /// assigns both — including clearing `effort` when the supplied value
    /// does not set it. Every setter on this builder assigns exactly the
    /// slots it names, so ordering is predictable:
    ///
    /// ```
    /// use clauders::messages::{MessageRequest, structured_outputs::OutputConfig};
    /// use clauders::types::{EffortLevel, MaxTokens, ModelId};
    ///
    /// // output_config() last: it assigns both slots, so effort is cleared.
    /// let req = MessageRequest::builder()
    ///     .model(ModelId::claude_sonnet_5())
    ///     .max_tokens(MaxTokens::new(64))
    ///     .effort(EffortLevel::High)
    ///     .output_config(OutputConfig::json_schema(serde_json::json!({"type": "object"})))
    ///     .build();
    /// let j = serde_json::to_value(&req).unwrap();
    /// assert!(j["output_config"].get("effort").is_none());
    ///
    /// // effort() last: it names only its own slot, so both survive.
    /// let req = MessageRequest::builder()
    ///     .model(ModelId::claude_sonnet_5())
    ///     .max_tokens(MaxTokens::new(64))
    ///     .output_config(OutputConfig::json_schema(serde_json::json!({"type": "object"})))
    ///     .effort(EffortLevel::High)
    ///     .build();
    /// let j = serde_json::to_value(&req).unwrap();
    /// assert_eq!(j["output_config"]["effort"], "high");
    /// ```
    #[must_use]
    pub fn output_config(mut self, c: crate::messages::structured_outputs::OutputConfig) -> Self {
        self.fields.output_format = c.format;
        self.fields.effort = c.effort;
        self
    }

    /// Set how much reasoning effort the model should spend.
    ///
    /// Assigns only the effort slot, leaving any output format untouched.
    #[must_use]
    pub const fn effort(mut self, effort: crate::types::EffortLevel) -> Self {
        self.fields.effort = Some(effort);
        self
    }

    /// Set the extended-thinking configuration.
    #[must_use]
    pub const fn thinking(mut self, thinking: crate::messages::thinking::ThinkingConfig) -> Self {
        self.fields.thinking = Some(thinking);
        self
    }

    /// Set the service-tier selector.
    #[must_use]
    pub const fn service_tier(mut self, tier: RequestServiceTier) -> Self {
        self.fields.service_tier = Some(tier);
        self
    }

    /// Set the geographic inference region.
    #[must_use]
    pub fn inference_geo(mut self, geo: InferenceGeo) -> Self {
        self.fields.inference_geo = Some(geo);
        self
    }

    /// Set the code-execution container to reuse.
    #[must_use]
    pub fn container(mut self, container: ContainerId) -> Self {
        self.fields.container = Some(container);
        self
    }

    /// Set the top-level cache-control breakpoint.
    #[must_use]
    pub const fn cache_control(mut self, cache_control: crate::types::CacheControl) -> Self {
        self.fields.cache_control = Some(cache_control);
        self
    }
}

impl MessageRequestBuilder<Present, Present> {
    /// Build the [`MessageRequest`].
    ///
    /// Only callable when both `model` and `max_tokens` have been supplied
    /// (enforced at compile time via type-state).
    ///
    /// # Panics
    /// Does not panic: the type-state `Present` guarantees both required
    /// fields are set. The `expect` calls are unreachable safety nets.
    #[must_use]
    pub fn build(self) -> MessageRequest {
        #[expect(
            clippy::expect_used,
            reason = "type-state Present guarantees model is set; this branch is unreachable"
        )]
        let model = self
            .fields
            .model
            .expect("invariant: type-state Present guarantees model is set");
        #[expect(
            clippy::expect_used,
            reason = "type-state Present guarantees max_tokens is set; this branch is unreachable"
        )]
        let max_tokens = self
            .fields
            .max_tokens
            .expect("invariant: type-state Present guarantees max_tokens is set");

        let output_config = match (self.fields.output_format, self.fields.effort) {
            (None, None) => None,
            (format, effort) => {
                Some(crate::messages::structured_outputs::OutputConfig { format, effort })
            }
        };

        MessageRequest {
            model,
            max_tokens,
            messages: self.fields.messages,
            system: self.fields.system,
            temperature: self.fields.temperature,
            top_p: self.fields.top_p,
            top_k: self.fields.top_k,
            stop_sequences: self.fields.stop_sequences,
            metadata: self.fields.metadata,
            tools: self.fields.tools,
            tool_choice: self.fields.tool_choice,
            output_config,
            thinking: self.fields.thinking,
            service_tier: self.fields.service_tier,
            inference_geo: self.fields.inference_geo,
            container: self.fields.container,
            cache_control: self.fields.cache_control,
            stream: false,
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
    use crate::messages::content::{ContentBlockParam, TextBlock};
    use crate::messages::structured_outputs::OutputConfig;
    use crate::messages::thinking::{ThinkingConfig, ThinkingDisplay};
    use crate::types::{
        EffortLevel, MaxTokens, ModelId, StopSequence, SystemPrompt, Temperature, TopK, TopP,
        UserId,
    };

    /// A builder with the two required slots (model + `max_tokens`) filled,
    /// for the many tests that only vary the optional fields.
    fn base_builder() -> MessageRequestBuilder<Present, Present> {
        MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64))
    }

    #[test]
    fn builder_round_trips_minimal_request() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(1024))
            .add_user_text("Hello, Claude")
            .build();

        assert_eq!(req.model, ModelId::claude_sonnet_4_5());
        assert_eq!(req.max_tokens.get(), 1024);
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, Role::User);
    }

    #[test]
    fn serializes_minimal_request_correctly() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(1024))
            .add_user_text("Hello, Claude")
            .build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();

        assert_eq!(j["model"], "claude-sonnet-4-5");
        assert_eq!(j["max_tokens"], 1024);
        assert_eq!(j["messages"][0]["role"], "user");
        assert_eq!(j["messages"][0]["content"], "Hello, Claude");
        assert!(j.get("stream").is_none());
    }

    #[test]
    #[expect(
        deprecated,
        reason = "pins the wire shape of parameters that are deprecated but still serialized"
    )]
    fn serializes_fully_populated_request_with_all_optional_fields_present() {
        let user_id = UserId::new("user-42").unwrap();
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(512))
            .system(SystemPrompt::text("You are terse."))
            .temperature(Temperature::new(0.7).unwrap())
            .top_p(TopP::new(0.9).unwrap())
            .top_k(TopK::new(40).unwrap())
            .stop_sequences(vec![StopSequence::new("STOP").unwrap()])
            .metadata(Metadata {
                user_id: Some(user_id),
            })
            .add_user_text("Hello")
            .build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();

        assert!(j.get("system").is_some(), "system must be present");
        assert_eq!(j["system"], "You are terse.");
        assert!(
            j.get("temperature").is_some(),
            "temperature must be present"
        );
        assert!((j["temperature"].as_f64().unwrap() - 0.7_f64).abs() < 1e-6);
        assert!(j.get("top_p").is_some(), "top_p must be present");
        assert!((j["top_p"].as_f64().unwrap() - 0.9_f64).abs() < 1e-6);
        assert!(j.get("top_k").is_some(), "top_k must be present");
        assert_eq!(j["top_k"], 40);
        assert!(
            j.get("stop_sequences").is_some(),
            "stop_sequences must be present"
        );
        assert_eq!(j["stop_sequences"], serde_json::json!(["STOP"]));
        assert!(j.get("metadata").is_some(), "metadata must be present");
        assert_eq!(j["metadata"]["user_id"], "user-42");
    }

    #[test]
    fn serializes_minimal_request_omits_all_optional_fields() {
        let req = base_builder().add_user_text("Hi").build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();

        assert!(j.get("system").is_none(), "system must be absent");
        assert!(j.get("temperature").is_none(), "temperature must be absent");
        assert!(j.get("top_p").is_none(), "top_p must be absent");
        assert!(j.get("top_k").is_none(), "top_k must be absent");
        assert!(j.get("metadata").is_none(), "metadata must be absent");
        assert!(
            j.get("stop_sequences").is_none(),
            "stop_sequences must be absent when empty"
        );
    }

    #[test]
    fn add_system_text_emits_system_role_on_the_wire() {
        let req = base_builder().add_system_text("operator note").build();
        let j = serde_json::to_string(&req).unwrap();
        assert!(j.contains(r#""role":"system""#), "got: {j}");
    }

    #[test]
    fn system_role_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Role::System).unwrap(), r#""system""#);
    }

    #[test]
    fn request_service_tier_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&RequestServiceTier::Auto).unwrap(),
            r#""auto""#
        );
        assert_eq!(
            serde_json::to_string(&RequestServiceTier::StandardOnly).unwrap(),
            r#""standard_only""#
        );
    }

    #[test]
    fn inference_geo_and_container_id_serialize_transparently() {
        assert_eq!(
            serde_json::to_string(&InferenceGeo::new("us")).unwrap(),
            r#""us""#
        );
        assert_eq!(
            serde_json::to_string(&ContainerId::new("c_42")).unwrap(),
            r#""c_42""#
        );
    }

    #[test]
    fn stop_sequences_accepts_array_literal() {
        // Verify that an array (not Vec) is accepted by the setter.
        // This compiles only if the setter takes impl IntoIterator.
        let req = base_builder()
            .stop_sequences([StopSequence::new("STOP").unwrap()])
            .add_user_text("Hi")
            .build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(j["stop_sequences"], serde_json::json!(["STOP"]));
    }

    #[test]
    fn output_config_serializes_when_set() {
        use crate::messages::structured_outputs::OutputConfig;

        let schema = serde_json::json!({"type": "object"});
        let req = base_builder()
            .output_config(OutputConfig::json_schema(schema.clone()))
            .add_user_text("Hi")
            .build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(
            j["output_config"]["format"]["type"], "json_schema",
            "output_config.format.type must be 'json_schema'"
        );
        assert_eq!(
            j["output_config"]["format"]["schema"], schema,
            "output_config.format.schema must carry the schema verbatim"
        );
    }

    #[test]
    fn output_config_omitted_when_unset() {
        let req = base_builder().add_user_text("Hi").build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert!(
            j.get("output_config").is_none(),
            "output_config must be absent when not set"
        );
    }

    #[test]
    fn output_config_survives_model_and_max_tokens_transitions() {
        // Regression guard: setting output_config before .model() and
        // .max_tokens() must not lose the value through the type-state
        // transitions.
        use crate::messages::structured_outputs::OutputConfig;

        let schema = serde_json::json!({"type": "object", "properties": {}});
        let req = MessageRequest::builder()
            .output_config(OutputConfig::json_schema(schema.clone()))
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64))
            .add_user_text("Hi")
            .build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(
            j["output_config"]["format"]["type"], "json_schema",
            "output_config must survive model() and max_tokens() transitions"
        );
        assert_eq!(
            j["output_config"]["format"]["schema"], schema,
            "output_config schema must survive transitions unchanged"
        );
    }

    #[test]
    fn effort_alone_produces_an_output_config() {
        let req = base_builder()
            .effort(EffortLevel::High)
            .add_user_text("Hi")
            .build();

        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["output_config"], serde_json::json!({"effort": "high"}));
    }

    #[test]
    fn output_config_is_absent_when_neither_slot_is_set() {
        let req = base_builder().add_user_text("Hi").build();

        let j = serde_json::to_value(&req).unwrap();
        assert!(
            j.get("output_config").is_none(),
            "an empty output_config must be omitted, never sent as {{}}"
        );
    }

    #[test]
    fn output_config_after_effort_assigns_both_slots_and_clears_effort() {
        let req = base_builder()
            .effort(EffortLevel::High)
            .output_config(OutputConfig::json_schema(
                serde_json::json!({"type": "object"}),
            ))
            .add_user_text("Hi")
            .build();

        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["output_config"]["format"]["type"], "json_schema");
        assert!(
            j["output_config"].get("effort").is_none(),
            "output_config() names both slots, so it assigns both: effort is cleared"
        );
    }

    #[test]
    fn effort_after_output_config_keeps_both() {
        let req = base_builder()
            .output_config(OutputConfig::json_schema(
                serde_json::json!({"type": "object"}),
            ))
            .effort(EffortLevel::High)
            .add_user_text("Hi")
            .build();

        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["output_config"]["format"]["type"], "json_schema");
        assert_eq!(j["output_config"]["effort"], "high");
    }

    #[test]
    fn output_config_carrying_both_halves_survives_intact() {
        let req = base_builder()
            .output_config(
                OutputConfig::json_schema(serde_json::json!({"type": "object"}))
                    .with_effort(EffortLevel::Max),
            )
            .add_user_text("Hi")
            .build();

        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["output_config"]["format"]["type"], "json_schema");
        assert_eq!(j["output_config"]["effort"], "max");
    }

    #[test]
    fn thinking_is_absent_when_unset() {
        let req = base_builder().add_user_text("Hi").build();

        let j = serde_json::to_value(&req).unwrap();
        assert!(
            j.get("thinking").is_none(),
            "thinking must be absent when never set"
        );
    }

    #[test]
    fn thinking_reaches_the_request_body() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(4096))
            .thinking(ThinkingConfig::enabled_with_display(
                1024,
                ThinkingDisplay::Omitted,
            ))
            .add_user_text("Hi")
            .build();

        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["thinking"]["type"], "enabled");
        assert_eq!(j["thinking"]["budget_tokens"], 1024);
        assert_eq!(j["thinking"]["display"], "omitted");
    }

    #[test]
    fn a_batched_request_carries_thinking() {
        use crate::messages::BatchRequest;
        use crate::types::CustomRequestId;

        let batch = BatchRequest::builder()
            .add(
                CustomRequestId::new("r1").unwrap(),
                MessageRequest::builder()
                    .model(ModelId::claude_sonnet_4_5())
                    .max_tokens(MaxTokens::new(4096))
                    .thinking(ThinkingConfig::adaptive_with_display(
                        ThinkingDisplay::Omitted,
                    ))
                    .add_user_text("Hi")
                    .build(),
            )
            .build();

        let j = serde_json::to_value(&batch).unwrap();
        let params = &j["requests"][0]["params"];
        assert_eq!(
            params["thinking"]["type"], "adaptive",
            "thinking must reach the batch path through the wrapped MessageRequest"
        );
        assert_eq!(params["thinking"]["display"], "omitted");
    }

    #[test]
    fn ga_request_params_serialize_when_set() {
        let req = base_builder()
            .service_tier(RequestServiceTier::StandardOnly)
            .inference_geo(InferenceGeo::new("us"))
            .container(ContainerId::new("c_1"))
            .cache_control(crate::types::CacheControl::ephemeral())
            .build();
        let j = serde_json::to_string(&req).unwrap();
        assert!(j.contains(r#""service_tier":"standard_only""#), "got: {j}");
        assert!(j.contains(r#""inference_geo":"us""#), "got: {j}");
        assert!(j.contains(r#""container":"c_1""#), "got: {j}");
        assert!(
            j.contains(r#""cache_control":{"type":"ephemeral"}"#),
            "got: {j}"
        );
    }

    #[test]
    fn ga_request_params_are_skipped_when_unset() {
        let j = serde_json::to_string(&base_builder().build()).unwrap();
        assert!(!j.contains("service_tier"), "got: {j}");
        assert!(!j.contains("inference_geo"), "got: {j}");
        assert!(!j.contains("container"), "got: {j}");
        assert!(!j.contains("cache_control"), "got: {j}");
    }

    #[test]
    fn message_content_blocks_serializes_as_json_array() {
        let req = base_builder()
            .add_message(
                Role::User,
                MessageContent::Blocks(vec![ContentBlockParam::Text(TextBlock::new("x"))]),
            )
            .build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();

        let content = &j["messages"][0]["content"];
        assert!(
            content.is_array(),
            "content must be a JSON array for Blocks variant"
        );
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "x");
    }

    #[test]
    fn request_with_image_and_document_blocks_serializes() {
        use crate::messages::content::document::{DocumentBlock, DocumentSource, PdfMediaType};
        use crate::messages::content::image::{ImageBlock, ImageMediaType, ImageSource};
        use crate::messages::{ContentBlockParam, InputMessage, MessageContent, Role};

        let msg = InputMessage {
            role: Role::User,
            content: MessageContent::Blocks(vec![
                ContentBlockParam::Image(ImageBlock {
                    source: ImageSource::Base64 {
                        media_type: ImageMediaType::Png,
                        data: "AAAA".into(),
                    },
                    cache_control: None,
                }),
                ContentBlockParam::Document(DocumentBlock {
                    source: DocumentSource::Base64 {
                        media_type: PdfMediaType::ApplicationPdf,
                        data: "JVBER".into(),
                    },
                    cache_control: None,
                    citations: None,
                    context: None,
                    title: None,
                }),
            ]),
        };
        let j = serde_json::to_value(&msg).unwrap();
        assert_eq!(j["content"][0]["type"], "image");
        assert_eq!(j["content"][1]["type"], "document");
    }
}
