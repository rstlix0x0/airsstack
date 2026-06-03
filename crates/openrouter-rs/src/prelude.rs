//! Common imports for `use openrouter_rs::prelude::*;`.
//!
//! Re-exports the types most callers name to build a request or read a
//! response across every capability — chat, tools, structured outputs,
//! provider routing, caching, and the model catalog. A single glob import
//! covers the common surface; less-common types remain reachable from the
//! crate root and their modules.
//!
//! Pure re-export module: it declares no items of its own and carries no
//! logic, so it has no inline tests.

pub use crate::error::Error;

pub use crate::client::Client;

pub use crate::types::{
    ApiKey, FrequencyPenalty, FunctionName, MaxTokens, ModelId, PresencePenalty, Price,
    PricePerToken, ProviderSlug, RepetitionPenalty, SchemaName, Seed, StopSequences, Temperature,
    ToolCallId, TopK, TopP,
};

pub use crate::chat::{
    CacheClear, CacheControl, CacheKind, CacheMode, CacheStatus, CacheTtl, CacheTtlSeconds, Cached,
    ChatCompletion, ChatRequest, DataCollection, FallbackPolicy, FinishReason, FunctionCall,
    FunctionDef, JsonSchemaConfig, LatencyCeiling, MaxPrice, Message, ParameterRequirement,
    ProviderPreferences, ProviderPreferencesBuilder, ProviderSort, Quantization, ResponseCache,
    ResponseFormat, Role, SchemaStrictness, ThroughputFloor, Tool, ToolCall, ToolChoice, ToolType,
    Usage, ZeroDataRetention,
};

pub use crate::models::{Model, Pricing};

#[cfg(feature = "streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
pub use crate::chat::{ChatStream, StreamChunk};

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub use crate::client::DefaultClient;
