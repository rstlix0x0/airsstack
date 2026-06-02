//! The type-state builder for [`crate::chat::ChatRequest`].
//!
//! Exists so the required `model` and `messages` fields are enforced at compile
//! time: `build()` is implemented only when both states are [`Present`]. The
//! optional sampling setters are available in any state.
//!
//! Responsibilities:
//! - State markers [`Missing`] / [`Present`] and the sealed [`FieldState`] trait.
//! - [`ChatRequestBuilder`] and its setters / `build`.

use std::marker::PhantomData;

use crate::chat::message::Message;
use crate::chat::request::ChatRequest;
use crate::chat::tool::{Tool, ToolChoice};
use crate::types::{
    FrequencyPenalty, MaxTokens, ModelId, PresencePenalty, RepetitionPenalty, Seed, StopSequences,
    Temperature, TopK, TopP,
};

mod sealed {
    pub trait Sealed {}
}

/// Marker trait for a builder field's set/unset state. Sealed — downstream
/// crates cannot add states.
pub trait FieldState: sealed::Sealed {}

/// Type-state marker: the field has not been set.
#[derive(Debug)]
pub struct Missing;

/// Type-state marker: the field has been set.
#[derive(Debug)]
pub struct Present;

impl sealed::Sealed for Missing {}
impl sealed::Sealed for Present {}
impl FieldState for Missing {}
impl FieldState for Present {}

/// All mutable builder data, in one non-generic struct so a required-field
/// transition can move the whole value without enumerating fields.
#[derive(Clone, Debug, Default)]
struct ChatRequestFields {
    model: Option<ModelId>,
    messages: Option<Vec<Message>>,
    max_tokens: Option<MaxTokens>,
    temperature: Option<Temperature>,
    top_p: Option<TopP>,
    top_k: Option<TopK>,
    seed: Option<Seed>,
    frequency_penalty: Option<FrequencyPenalty>,
    presence_penalty: Option<PresencePenalty>,
    repetition_penalty: Option<RepetitionPenalty>,
    stop: Option<StopSequences>,
    user: Option<String>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<ToolChoice>,
}

/// Builds a [`ChatRequest`]; `M` tracks the `model` state, `Ms` the `messages`
/// state. `build()` exists only when both are [`Present`].
#[derive(Debug)]
pub struct ChatRequestBuilder<M, Ms>
where
    M: FieldState,
    Ms: FieldState,
{
    fields: ChatRequestFields,
    _markers: PhantomData<(M, Ms)>,
}

impl ChatRequestBuilder<Missing, Missing> {
    /// Start a fresh builder with no fields set.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            fields: ChatRequestFields::default(),
            _markers: PhantomData,
        }
    }
}

impl<Ms: FieldState> ChatRequestBuilder<Missing, Ms> {
    /// Set the required target model, transitioning the model state to `Present`.
    #[must_use]
    pub fn model(self, model: ModelId) -> ChatRequestBuilder<Present, Ms> {
        ChatRequestBuilder {
            fields: ChatRequestFields {
                model: Some(model),
                ..self.fields
            },
            _markers: PhantomData,
        }
    }
}

impl<M: FieldState> ChatRequestBuilder<M, Missing> {
    /// Set the required messages, transitioning the messages state to `Present`.
    #[must_use]
    pub fn messages(self, messages: Vec<Message>) -> ChatRequestBuilder<M, Present> {
        ChatRequestBuilder {
            fields: ChatRequestFields {
                messages: Some(messages),
                ..self.fields
            },
            _markers: PhantomData,
        }
    }
}

impl<M: FieldState, Ms: FieldState> ChatRequestBuilder<M, Ms> {
    /// Cap the number of generated tokens.
    #[must_use]
    pub const fn max_tokens(mut self, v: MaxTokens) -> Self {
        self.fields.max_tokens = Some(v);
        self
    }

    /// Set the sampling temperature.
    #[must_use]
    pub const fn temperature(mut self, v: Temperature) -> Self {
        self.fields.temperature = Some(v);
        self
    }

    /// Set nucleus-sampling `top_p`.
    #[must_use]
    pub const fn top_p(mut self, v: TopP) -> Self {
        self.fields.top_p = Some(v);
        self
    }

    /// Set top-k sampling.
    #[must_use]
    pub const fn top_k(mut self, v: TopK) -> Self {
        self.fields.top_k = Some(v);
        self
    }

    /// Set the RNG seed for (best-effort) reproducible sampling.
    #[must_use]
    pub const fn seed(mut self, v: Seed) -> Self {
        self.fields.seed = Some(v);
        self
    }

    /// Set the frequency penalty.
    #[must_use]
    pub const fn frequency_penalty(mut self, v: FrequencyPenalty) -> Self {
        self.fields.frequency_penalty = Some(v);
        self
    }

    /// Set the presence penalty.
    #[must_use]
    pub const fn presence_penalty(mut self, v: PresencePenalty) -> Self {
        self.fields.presence_penalty = Some(v);
        self
    }

    /// Set the repetition penalty.
    #[must_use]
    pub const fn repetition_penalty(mut self, v: RepetitionPenalty) -> Self {
        self.fields.repetition_penalty = Some(v);
        self
    }

    /// Set stop sequences.
    #[must_use]
    pub fn stop(mut self, stop: StopSequences) -> Self {
        self.fields.stop = Some(stop);
        self
    }

    /// Set an opaque end-user identifier for abuse monitoring.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.fields.user = Some(user.into());
        self
    }

    /// Set the list of tools the model may call.
    #[must_use]
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.fields.tools = Some(tools);
        self
    }

    /// Control which tool, if any, the model calls.
    #[must_use]
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.fields.tool_choice = Some(choice);
        self
    }
}

impl ChatRequestBuilder<Present, Present> {
    /// Assemble the validated request. Every field value is an already-validated
    /// newtype; the method is infallible at runtime.
    ///
    /// # Panics
    ///
    /// Panics only if the type-state machinery is somehow bypassed — that is,
    /// if `build` is called on a builder whose `model` or `messages` `Option`
    /// is unexpectedly `None`. Under normal usage this cannot happen: the
    /// `Present` type state guarantees both were set.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "the Present type state proves model and messages were set"
    )]
    pub fn build(self) -> ChatRequest {
        let f = self.fields;
        ChatRequest {
            model: f.model.expect("model is Present"),
            messages: f.messages.expect("messages is Present"),
            max_tokens: f.max_tokens,
            temperature: f.temperature,
            top_p: f.top_p,
            top_k: f.top_k,
            seed: f.seed,
            frequency_penalty: f.frequency_penalty,
            presence_penalty: f.presence_penalty,
            repetition_penalty: f.repetition_penalty,
            stop: f.stop,
            user: f.user,
            tools: f.tools,
            tool_choice: f.tool_choice,
            stream: false,
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "tests unwrap/expect known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use crate::chat::tool::{FunctionDef, ToolChoice, ToolType};
    use crate::types::{FunctionName, StopSequences};
    use serde_json::json;

    fn model() -> ModelId {
        ModelId::custom("openai/gpt-4o").unwrap()
    }

    // f32 values serialize with finite precision; compare as f64 with a small
    // tolerance rather than asserting exact JSON equality.
    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-4
    }

    fn json_f64(v: &serde_json::Value, key: &str) -> f64 {
        v[key].as_f64().expect("value must be a JSON number")
    }

    #[test]
    fn optional_fields_survive_required_field_transitions() {
        // Set an optional BEFORE both required fields, then transition both
        // required states. The field-move shape must preserve the optional.
        let req = ChatRequest::builder()
            .temperature(Temperature::new(0.7).unwrap())
            .model(model())
            .messages(vec![Message::user("hi")])
            .max_tokens(MaxTokens::new(256).unwrap())
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert!(approx_eq(json_f64(&v, "temperature"), 0.7));
        assert_eq!(v["max_tokens"], json!(256));
    }

    #[test]
    fn build_emits_all_set_sampling_params() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .top_p(TopP::new(0.9).unwrap())
            .top_k(TopK::new(40))
            .seed(Seed::new(7))
            .frequency_penalty(FrequencyPenalty::new(0.5).unwrap())
            .presence_penalty(PresencePenalty::new(-0.5).unwrap())
            .repetition_penalty(RepetitionPenalty::new(1.1).unwrap())
            .stop(StopSequences::new(vec!["\n\n".to_owned()]).expect("valid"))
            .user("user-123")
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert!(approx_eq(json_f64(&v, "top_p"), 0.9));
        assert_eq!(v["top_k"], json!(40));
        assert_eq!(v["seed"], json!(7));
        assert!(approx_eq(json_f64(&v, "frequency_penalty"), 0.5));
        assert!(approx_eq(json_f64(&v, "presence_penalty"), -0.5));
        assert!(approx_eq(json_f64(&v, "repetition_penalty"), 1.1));
        assert_eq!(v["stop"], json!(["\n\n"]));
        assert_eq!(v["user"], json!("user-123"));
    }

    #[test]
    fn messages_then_model_order_also_builds() {
        // Order independence: messages first, then model.
        let req = ChatRequest::builder()
            .messages(vec![Message::user("hi")])
            .model(model())
            .build();
        assert_eq!(req.model().as_str(), "openai/gpt-4o");
    }

    #[test]
    fn tools_and_tool_choice_survive_required_field_transitions() {
        // Set optional tool fields BEFORE both required fields; the field-move
        // shape must preserve them through the type-state transitions.
        let tool = Tool::function(FunctionDef::new(FunctionName::new("fn1").unwrap()));
        let req = ChatRequest::builder()
            .tools(vec![tool])
            .tool_choice(ToolChoice::Auto)
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["tool_choice"], json!("auto"));
        assert_eq!(v["tools"][0]["type"], json!("function"));
    }

    #[test]
    fn tool_choice_function_variant_serializes_correctly_from_builder() {
        let fn_name = FunctionName::new("search_books").unwrap();
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .tool_choice(ToolChoice::Function { name: fn_name })
            .build();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["tool_choice"]["type"], json!("function"));
        assert_eq!(v["tool_choice"]["function"]["name"], json!("search_books"));
    }

    #[test]
    fn builder_without_tools_produces_no_tool_fields() {
        let req = ChatRequest::builder()
            .model(model())
            .messages(vec![Message::user("hi")])
            .build();
        // Verify the type fields are None at the struct level, not just serialization.
        assert!(req.tools.is_none());
        assert!(req.tool_choice.is_none());
    }

    #[test]
    fn tool_type_function_serializes() {
        assert_eq!(
            serde_json::to_value(ToolType::Function).unwrap(),
            json!("function")
        );
    }
}
