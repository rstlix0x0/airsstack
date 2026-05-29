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

use crate::messages::content::ContentBlock;
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
    Blocks(Vec<ContentBlock>),
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
///     .model(ModelId::claude_sonnet_4_5())
///     .max_tokens(MaxTokens::new(1024).unwrap())
///     .add_user_text("Hello, Claude")
///     .build();
///
/// assert_eq!(req.model.as_str(), "claude-sonnet-4-5");
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
    model: Option<ModelId>,
    max_tokens: Option<MaxTokens>,
    messages: Vec<InputMessage>,
    system: Option<SystemPrompt>,
    temperature: Option<Temperature>,
    top_p: Option<TopP>,
    top_k: Option<TopK>,
    stop_sequences: Vec<StopSequence>,
    metadata: Option<Metadata>,
    _m: PhantomData<M>,
    _mt: PhantomData<Mt>,
}

impl MessageRequestBuilder<Missing, Missing> {
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
        MessageRequestBuilder {
            model: Some(model),
            max_tokens: self.max_tokens,
            messages: self.messages,
            system: self.system,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stop_sequences: self.stop_sequences,
            metadata: self.metadata,
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
        MessageRequestBuilder {
            model: self.model,
            max_tokens: Some(max_tokens),
            messages: self.messages,
            system: self.system,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stop_sequences: self.stop_sequences,
            metadata: self.metadata,
            _m: self._m,
            _mt: PhantomData,
        }
    }
}

impl<M: sealed::BuilderModelState, Mt: sealed::BuilderMaxTokensState> MessageRequestBuilder<M, Mt> {
    /// Append a user-role plain-text message.
    #[must_use]
    pub fn add_user_text(mut self, text: impl Into<String>) -> Self {
        self.messages.push(InputMessage {
            role: Role::User,
            content: MessageContent::Text(text.into()),
        });
        self
    }

    /// Append an assistant-role plain-text message.
    #[must_use]
    pub fn add_assistant_text(mut self, text: impl Into<String>) -> Self {
        self.messages.push(InputMessage {
            role: Role::Assistant,
            content: MessageContent::Text(text.into()),
        });
        self
    }

    /// Append a message with an explicit role and content.
    #[must_use]
    pub fn add_message(mut self, role: Role, content: MessageContent) -> Self {
        self.messages.push(InputMessage { role, content });
        self
    }

    /// Set a system prompt.
    #[must_use]
    pub fn system(mut self, system: SystemPrompt) -> Self {
        self.system = Some(system);
        self
    }

    /// Set the sampling temperature.
    #[must_use]
    pub const fn temperature(mut self, temperature: Temperature) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the top-p nucleus sampling probability.
    #[must_use]
    pub const fn top_p(mut self, top_p: TopP) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set the top-k sampling parameter.
    #[must_use]
    pub const fn top_k(mut self, top_k: TopK) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Set stop sequences.
    ///
    /// Accepts any iterable — `Vec`, array literal, or iterator — of
    /// [`StopSequence`] values. The collected sequences replace any previously
    /// set value.
    #[must_use]
    pub fn stop_sequences(mut self, ss: impl IntoIterator<Item = StopSequence>) -> Self {
        self.stop_sequences = ss.into_iter().collect();
        self
    }

    /// Set per-request metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
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
            .model
            .expect("invariant: type-state Present guarantees model is set");
        #[expect(
            clippy::expect_used,
            reason = "type-state Present guarantees max_tokens is set; this branch is unreachable"
        )]
        let max_tokens = self
            .max_tokens
            .expect("invariant: type-state Present guarantees max_tokens is set");

        MessageRequest {
            model,
            max_tokens,
            messages: self.messages,
            system: self.system,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stop_sequences: self.stop_sequences,
            metadata: self.metadata,
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
    use crate::messages::content::{ContentBlock, TextBlock};
    use crate::types::{
        MaxTokens, ModelId, StopSequence, SystemPrompt, Temperature, TopK, TopP, UserId,
    };

    #[test]
    fn builder_round_trips_minimal_request() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(1024).unwrap())
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
            .max_tokens(MaxTokens::new(1024).unwrap())
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
    fn serializes_fully_populated_request_with_all_optional_fields_present() {
        let user_id = UserId::new("user-42").unwrap();
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(512).unwrap())
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
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .add_user_text("Hi")
            .build();

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
    fn stop_sequences_accepts_array_literal() {
        // Verify that an array (not Vec) is accepted by the setter.
        // This compiles only if the setter takes impl IntoIterator.
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .stop_sequences([StopSequence::new("STOP").unwrap()])
            .add_user_text("Hi")
            .build();

        let j: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(j["stop_sequences"], serde_json::json!(["STOP"]));
    }

    #[test]
    fn message_content_blocks_serializes_as_json_array() {
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .add_message(
                Role::User,
                MessageContent::Blocks(vec![ContentBlock::Text(TextBlock::new("x"))]),
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
}
