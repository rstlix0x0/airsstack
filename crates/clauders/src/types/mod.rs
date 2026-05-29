//! Strongly-typed domain primitives.
//!
//! Each primitive is a newtype with a validating constructor: invariants
//! are enforced once at construction and downstream code trusts the type
//! as proof. Constructors return per-type `Invalid*` errors implementing
//! [`std::error::Error`].

mod api_key;
mod base_url;
#[cfg(feature = "messages-batches")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-batches")))]
mod batch_id;
#[cfg(feature = "messages-caching")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-caching")))]
pub mod caching;
#[cfg(feature = "messages-batches")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-batches")))]
mod custom_request_id;
mod ids;
mod model_id;
mod numeric;
mod system;
mod version;

pub use api_key::{ApiKey, InvalidApiKey};
pub use base_url::{BaseUrl, InvalidBaseUrl};
#[cfg(feature = "messages-batches")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-batches")))]
pub use batch_id::{BatchId, InvalidBatchId};
#[cfg(feature = "messages-caching")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-caching")))]
pub use caching::{CacheControl, CacheTtl};
#[cfg(feature = "messages-batches")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-batches")))]
pub use custom_request_id::{CustomRequestId, InvalidCustomRequestId};
pub use ids::{
    InvalidMessageId, InvalidOrganizationId, InvalidRequestId, InvalidStopSequence,
    InvalidToolName, InvalidToolUseId, InvalidUserId, MessageId, OrganizationId, RequestId,
    StopSequence, ToolName, ToolUseId, UserId,
};
pub use model_id::{InvalidModelId, ModelId};
pub use numeric::{
    InvalidMaxTokens, InvalidTemperature, InvalidTopK, InvalidTopP, MaxTokens, Temperature, TopK,
    TopP,
};
pub use system::{SystemPrompt, SystemSegment, SystemSegmentKind};
pub use version::{AnthropicVersion, BetaHeader, InvalidAnthropicVersion, InvalidBetaHeader};
