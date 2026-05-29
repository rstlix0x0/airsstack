//! Unofficial Rust SDK for the Anthropic Claude Messages API.
//!
//! Provides strongly-typed wrappers around the `POST /v1/messages` surface:
//! request and response models, sampling parameters, system prompts, tool
//! use, prompt caching, streaming, message batches, and structured outputs.
//!
//! See [README](https://github.com/rstlix0x0/airsstack) for an overview.
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
pub mod retry;
pub mod transport;
pub mod types;
#[cfg(any(feature = "messages", feature = "models"))]
pub(crate) mod wire_helpers;

pub use auth::Auth;
pub use builder::{BuilderApiKeyState, ClientBuilder, Missing, Present};
pub use client::Client;
#[cfg(feature = "transport-reqwest")]
pub use client::DefaultClient;
pub use config::Config;
pub use error::{ApiError, ApiErrorBody, BuildError, Error, ErrorType, TransportError};
pub use retry::{ExpBackoff, InvalidExpBackoff, Jitter, RetryPolicy};
