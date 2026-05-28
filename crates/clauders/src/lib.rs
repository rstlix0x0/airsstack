//! Unofficial Rust SDK for the Anthropic Claude Messages API.
//!
//! Provides strongly-typed wrappers around the `POST /v1/messages` surface:
//! request and response models, sampling parameters, system prompts, tool
//! use, prompt caching, streaming, message batches, and structured outputs.
//!
//! See [README](https://github.com/rstlix0x0/airsstack) for an overview.
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub(crate) mod headers;
pub mod transport;
pub mod types;

pub use error::{ApiError, ApiErrorBody, BuildError, Error, ErrorType, TransportError};
