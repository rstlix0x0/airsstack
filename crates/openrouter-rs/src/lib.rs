//! Unofficial Rust SDK for the OpenRouter API.
//!
//! [OpenRouter](https://openrouter.ai) is a unified, OpenAI-compatible gateway
//! that routes chat-completion requests across many model providers behind a
//! single API and a single API key. This crate targets that API from Rust.
//!
//! This crate is not affiliated with OpenRouter.
//!
//! # Quick start
//!
//! ```no_run
//! use openrouter_rs::prelude::*;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
//! let client = Client::builder()?.api_key(api_key).build()?;
//!
//! let req = ChatRequest::builder()
//!     .model(ModelId::custom("openai/gpt-4o-mini")?)
//!     .messages(vec![Message::user("Say hi in one word.")])
//!     .build();
//!
//! let completion = client.chat().send(req).await?;
//! if let Some(choice) = completion.choices.first() {
//!     println!("{:?}", choice.message.content);
//! }
//! # Ok(())
//! # }
//! ```
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod auth;
pub mod chat;
pub mod client;
mod config;
mod headers;
pub mod models;
mod wire_helpers;

pub use auth::Auth;
pub use chat::{
    CacheClear, CacheControl, CacheKind, CacheMode, CacheStatus, CacheTtl, CacheTtlSeconds, Cached,
    ChatCompletion, ChatRequest, ChatResource, CompletionTokensDetails, CostDetails,
    DataCollection, FallbackPolicy, FinishReason, FunctionCall, FunctionDef, JsonSchemaConfig,
    LatencyCeiling, MaxPrice, Message, ParameterRequirement, PromptTokensDetails,
    ProviderPreferences, ProviderPreferencesBuilder, ProviderSort, Quantization, ResponseCache,
    ResponseFormat, Role, SchemaStrictness, ThroughputFloor, Tool, ToolCall, ToolChoice, ToolType,
    ZeroDataRetention,
};
pub use client::Client;
pub use config::Config;
pub use models::{Model, ModelsResource, Pricing};

#[cfg(feature = "streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
pub use chat::{ChatStream, StreamChunk};

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub use client::DefaultClient;

pub mod builder;
pub mod error;
pub mod prelude;
#[doc(inline)]
pub use airs_transport as transport;
pub mod types;
