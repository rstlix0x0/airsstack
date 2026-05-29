//! Messages API surface.
//!
//! Exists as a feature-gated module so the request and response types
//! for the `POST /v1/messages` endpoint are only compiled when the
//! `messages` Cargo feature is enabled (on by default).
//!
//! Responsibilities:
//! - Re-export all public types from `content`, `request`, `response`,
//!   and `resource` so callers import from `clauders::messages::*`
//!   without navigating sub-modules.
//! - Declare `MessagesResource` as the primary entry point returned by
//!   [`crate::client::Client::messages`].
//!
//! Not responsible for:
//! - HTTP transport — that is owned by [`crate::transport`].
//! - Client construction — that is the builder's job.
//!
//! Entry point: [`MessagesResource`], obtained via `client.messages()`.

pub mod content;
pub mod request;
pub mod resource;
pub mod response;

#[doc(inline)]
pub use content::{ContentBlock, TextBlock, ThinkingBlock};
#[doc(inline)]
pub use request::{
    InputMessage, MessageContent, MessageRequest, MessageRequestBuilder, Metadata, Role,
};
#[doc(inline)]
pub use resource::MessagesResource;
#[doc(inline)]
pub use response::{Message, MessageKind, StopReason, Usage};
