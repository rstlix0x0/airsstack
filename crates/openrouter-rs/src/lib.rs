//! Unofficial Rust SDK for the OpenRouter API.
//!
//! [OpenRouter](https://openrouter.ai) is a unified, OpenAI-compatible gateway
//! that routes chat-completion requests across many model providers behind a
//! single API and a single API key. This crate targets that API from Rust.
//!
//! This crate is not affiliated with OpenRouter.
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub mod types;
