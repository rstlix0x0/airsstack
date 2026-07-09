//! Unofficial Rust SDK for the Anthropic Claude Messages API.
//!
//! # Quick start
//!
//! ```no_run
//! # async fn run() -> Result<(), clauders::Error> {
//! use clauders::prelude::*;
//! let client = Client::builder()?
//!     .api_key(ApiKey::new(std::env::var("ANTHROPIC_API_KEY").unwrap()).unwrap())
//!     .build()?;
//! let req = MessageRequest::builder()
//!     .model(ModelId::claude_sonnet_4_5())
//!     .max_tokens(MaxTokens::new(1024).unwrap())
//!     .add_user_text("Say hi.")
//!     .build();
//! let msg = client.messages().create(req).await?;
//! println!("{:?}", msg.stop_reason);
//! # Ok(()) }
//! ```
//!
//! # Surface
//!
//! - [`messages::MessagesResource`] — request/response types and the
//!   `POST /v1/messages` entry point, including SSE streaming
//!   ([`messages::MessageStream`], [`messages::StreamEvent`]), tool
//!   (function-calling) types ([`messages::tools::Tool`],
//!   [`messages::tools::ToolChoice`], [`messages::tools::ToolUseBlock`]),
//!   prompt-caching fields, token counting, the Message Batches API, and
//!   structured outputs ([`messages::OutputConfig`]).
//! - [`models::ModelsResource`] — models resource (`GET /v1/models`).
//! - [`agent`] — the Claude Agent SDK surface for driving the `claude` Code
//!   CLI binary as a subprocess over the control protocol.
//!
//! The default HTTP transport is backed by `reqwest` with `rustls`.
//!
//! # Re-exports
//!
//! Core types are re-exported at the crate root (`clauders::Client`,
//! `clauders::Error`, etc.). The [`prelude`] module groups the most commonly
//! used imports so a single `use clauders::prelude::*;` covers most call sites.
#![forbid(unsafe_code)]

pub mod agent;
pub use agent::{ApiRuntime, CachePolicy};

/// Drive an agent session against OpenRouter's chat-completions API.
///
/// ```no_run
/// use clauders::agent::{OpenRouterRuntime, Options, Runtime};
/// use clauders::types::{MaxTokens, ModelId};
/// use openrouter_rs::Client;
/// use openrouter_rs::types::ApiKey;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::builder()?
///     .api_key(ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?)
///     .build()?;
/// let options = Options::builder()
///     .model(ModelId::custom("deepseek/deepseek-chat")?)
///     .max_tokens(MaxTokens::new(1024)?)
///     .build();
/// let runtime = OpenRouterRuntime::new(client, options)?;
/// let mut stream = runtime.run("Summarize this repo.".into()).await?;
/// # let _ = &mut stream;
/// # Ok(())
/// # }
/// ```
pub use agent::OpenRouterRuntime;

/// Route each agent turn to a backend model chosen by an LLM classifier.
///
/// ```no_run
/// use clauders::agent::{Options, RoutingRuntime, RoutingSummary, RuntimeClassifier};
/// use clauders::types::{MaxTokens, ModelId};
/// use clauders::OpenRouterRuntime;
/// use openrouter_rs::Client;
/// use openrouter_rs::types::ApiKey;
///
/// # fn make(model: &str) -> Result<OpenRouterRuntime, Box<dyn std::error::Error>> {
/// # let client = Client::builder()?
/// #     .api_key(ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?)
/// #     .build()?;
/// # let options = Options::builder()
/// #     .model(ModelId::custom(model)?)
/// #     .max_tokens(MaxTokens::new(1024)?)
/// #     .build();
/// # Ok(OpenRouterRuntime::new(client, options)?)
/// # }
/// # fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let judge = make("deepseek/deepseek-chat")?;
/// let cheap = make("deepseek/deepseek-chat")?;
/// let advanced = make("anthropic/claude-opus-4-7")?;
///
/// let routing = RoutingRuntime::builder(RuntimeClassifier::new(judge))
///     .target(cheap, RoutingSummary::new("cheap; routine edits and simple Q&A")?)
///     .fallback_target(advanced, RoutingSummary::new("advanced; hard reasoning")?)
///     .build()?;
/// # let _ = &routing;
/// # Ok(())
/// # }
/// ```
pub use agent::RoutingRuntime;

pub mod messages;

pub mod models;

pub mod auth;
pub mod builder;
pub mod client;
pub mod config;
pub mod error;
pub(crate) mod headers;
pub mod prelude;
pub mod retry;
#[cfg(test)]
mod test_support;
#[doc(inline)]
pub use airs_transport as transport;
pub mod types;
pub(crate) mod wire_helpers;

pub use auth::Auth;
pub use builder::{BuilderApiKeyState, ClientBuilder, Missing, Present};
pub use client::{Client, DefaultClient};
pub use config::Config;
pub use error::{ApiError, ApiErrorBody, BuildError, Error, ErrorType, TransportError};
pub use retry::{ExpBackoff, InvalidExpBackoff, Jitter, RetryPolicy};
