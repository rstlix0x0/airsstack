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
//!   tool definitions, the opaque [`ToolCallId`] for tool-call tracking,
//!   the validated [`SchemaName`] for structured-output schema names,
//!   the validated [`ProviderSlug`] for provider routing lists, and the
//!   non-negative [`Price`] for provider routing price limits.
//!
//! Each newtype's construction-failure reason is exported alongside it.

pub mod api_key;
pub mod base_url;
pub mod function_name;
pub mod model_id;
pub mod numeric;
pub mod price;
pub mod provider_slug;
pub mod schema_name;
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
pub use price::{InvalidPrice, Price};
pub use provider_slug::{InvalidProviderSlug, ProviderSlug};
pub use schema_name::{InvalidSchemaName, SchemaName};
pub use stop_sequences::{InvalidStopSequences, StopSequences};
pub use tool_call_id::{InvalidToolCallId, ToolCallId};
