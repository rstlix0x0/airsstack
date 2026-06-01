//! Unofficial Rust SDK for the OpenRouter API.
//!
//! [OpenRouter](https://openrouter.ai) is a unified, OpenAI-compatible gateway
//! that routes chat-completion requests across many model providers behind a
//! single API and a single API key. This crate targets that API from Rust.
//!
//! This crate is not affiliated with OpenRouter.
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod auth;
pub mod chat;
pub mod client;
mod config;
mod headers;
mod wire_helpers;

pub use auth::Auth;
pub use chat::{ChatCompletion, ChatRequest, ChatResource, Message, Role};
pub use client::Client;
pub use config::Config;

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub use client::DefaultClient;

pub mod builder;
pub mod error;
pub mod transport;
pub mod types;
