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

#[cfg(feature = "messages-batches")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-batches")))]
pub mod batches;

#[cfg(feature = "messages-streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-streaming")))]
pub mod streaming;

#[cfg(feature = "messages-tools")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-tools")))]
pub mod tools;

#[cfg(feature = "messages-token-counting")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-token-counting")))]
pub mod token_counting;

#[cfg(feature = "messages-structured-outputs")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-structured-outputs")))]
pub mod structured_outputs;

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

#[cfg(feature = "messages-streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-streaming")))]
#[doc(inline)]
pub use streaming::{ContentDelta, MessageMetaDelta, MessageStream, StreamEvent, UsageDelta};

#[cfg(feature = "messages-tools")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-tools")))]
#[doc(inline)]
pub use tools::{Tool, ToolChoice, ToolResultBlock, ToolResultContent, ToolUseBlock};

#[cfg(feature = "messages-token-counting")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-token-counting")))]
#[doc(inline)]
pub use token_counting::TokenCount;

#[cfg(feature = "messages-structured-outputs")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-structured-outputs")))]
#[doc(inline)]
pub use structured_outputs::{OutputConfig, OutputFormat};

#[cfg(feature = "messages-batches")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-batches")))]
#[doc(inline)]
pub use batches::{
    Batch, BatchKind, BatchList, BatchRequest, BatchRequestBuilder, BatchResult, BatchResultRow,
    BatchResultStream, BatchStatus, BatchedMessageRequest, BatchesResource, DeletedBatchKind,
    DeletedMessageBatch, RequestCounts,
};
