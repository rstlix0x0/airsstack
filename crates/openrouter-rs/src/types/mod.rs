//! Validated domain newtypes shared across the SDK surface.
//!
//! Each type parses-and-validates at construction, so request-building code
//! downstream trusts the invariant without re-checking it (parse, don't
//! validate). Identifiers and secrets are distinct types the compiler refuses
//! to swap; bounded numerics enforce their ranges in the constructor.
//!
//! Responsibilities:
//! - Re-export the secret [`ApiKey`], the validated [`BaseUrl`], the
//!   [`ModelId`] slug, the bounded sampling-parameter numerics, the
//!   capped [`StopSequences`] list, the validated [`FunctionName`] for
//!   tool definitions, and the opaque [`ToolCallId`] for tool-call tracking.
//!
//! Each newtype's construction-failure reason is exported alongside it.

pub mod api_key;
pub mod base_url;
pub mod function_name;
pub mod model_id;
pub mod numeric;
pub mod stop_sequences;
pub mod tool_call_id;

pub use api_key::{ApiKey, InvalidApiKey};
pub use base_url::{BaseUrl, InvalidBaseUrl};
pub use function_name::{FunctionName, InvalidFunctionName};
pub use model_id::{InvalidModelId, ModelId};
pub use numeric::{
    FrequencyPenalty, InvalidFrequencyPenalty, InvalidMaxTokens, InvalidPresencePenalty,
    InvalidRepetitionPenalty, InvalidTemperature, InvalidTopP, MaxTokens, PresencePenalty,
    RepetitionPenalty, Seed, Temperature, TopK, TopP,
};
pub use stop_sequences::{InvalidStopSequences, StopSequences};
pub use tool_call_id::{InvalidToolCallId, ToolCallId};
