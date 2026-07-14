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
