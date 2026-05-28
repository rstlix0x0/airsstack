//! Strongly-typed domain primitives.
//!
//! Each primitive is a newtype with a validating constructor: invariants
//! are enforced once at construction and downstream code trusts the type
//! as proof. Constructors return per-type `Invalid*` errors implementing
//! [`std::error::Error`].

mod api_key;
mod model_id;
mod numeric;
mod version;

pub use api_key::{ApiKey, InvalidApiKey};
pub use model_id::{InvalidModelId, ModelId};
pub use numeric::{
    InvalidMaxTokens, InvalidTemperature, InvalidTopK, InvalidTopP, MaxTokens, Temperature, TopK,
    TopP,
};
pub use version::{AnthropicVersion, BetaHeader, InvalidAnthropicVersion, InvalidBetaHeader};
