//! Unofficial Rust SDK for the Anthropic Claude Messages API.
//!
//! # Quick start
//!
//! ```no_run
//! # #[cfg(all(feature = "messages", feature = "transport-reqwest"))]
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
//! # Features
//!
//! Default features (enabled unless opted out):
//!
//! - `messages` — request/response types and [`messages::MessagesResource`] for
//!   `POST /v1/messages`.
//! - `messages-streaming` — SSE streaming via [`messages::MessageStream`] and
//!   [`messages::StreamEvent`].
//! - `messages-tools` — tool (function-calling) types: [`messages::tools::Tool`],
//!   [`messages::tools::ToolChoice`], [`messages::tools::ToolUseBlock`].
//! - `messages-caching` — prompt-caching fields on request types and
//!   cache-hit counters on [`messages::Usage`].
//! - `transport-reqwest` — default HTTP transport backed by `reqwest` with
//!   `rustls`.
//!
//! Optional features (disabled by default):
//!
//! - `messages-token-counting` — token-counting helper (`POST /v1/messages/count_tokens`).
//! - `messages-batches` — Message Batches API (`/v1/messages/batches`).
//! - `messages-structured-outputs` — constrain responses to a JSON Schema via
//!   [`messages::OutputConfig`].
//! - `models` — models resource (`GET /v1/models`).
//!
//! # Re-exports
//!
//! Core types are re-exported at the crate root (`clauders::Client`,
//! `clauders::Error`, etc.). The [`prelude`] module groups the most commonly
//! used imports so a single `use clauders::prelude::*;` covers most call sites.
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "messages")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages")))]
pub mod messages;

#[cfg(feature = "models")]
#[cfg_attr(docsrs, doc(cfg(feature = "models")))]
pub mod models;

pub mod auth;
pub mod builder;
pub mod client;
pub mod config;
pub mod error;
pub(crate) mod headers;
pub mod prelude;
pub mod retry;
pub mod transport;
pub mod types;
#[cfg(any(feature = "messages", feature = "models", feature = "messages-batches"))]
pub(crate) mod wire_helpers;

pub use auth::Auth;
pub use builder::{BuilderApiKeyState, ClientBuilder, Missing, Present};
pub use client::Client;
#[cfg(feature = "transport-reqwest")]
pub use client::DefaultClient;
pub use config::Config;
pub use error::{ApiError, ApiErrorBody, BuildError, Error, ErrorType, TransportError};
pub use retry::{ExpBackoff, InvalidExpBackoff, Jitter, RetryPolicy};
