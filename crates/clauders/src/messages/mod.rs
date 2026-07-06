//! Messages API surface.
//!
//! Exists as its own module tree so the request and response types for
//! the `POST /v1/messages` endpoint are scoped separately from the
//! mod.rs table of contents and the rest of the SDK surface.
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

pub mod batches;

pub mod streaming;

pub mod tools;

pub mod token_counting;

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

#[doc(inline)]
pub use streaming::{ContentDelta, MessageMetaDelta, MessageStream, StreamEvent, UsageDelta};

#[doc(inline)]
pub use tools::{Tool, ToolChoice, ToolResultBlock, ToolResultContent, ToolUseBlock};

#[doc(inline)]
pub use token_counting::TokenCount;

#[doc(inline)]
pub use structured_outputs::{OutputConfig, OutputFormat};

#[doc(inline)]
pub use batches::{
    Batch, BatchKind, BatchList, BatchRequest, BatchRequestBuilder, BatchResult, BatchResultRow,
    BatchResultStream, BatchStatus, BatchedMessageRequest, BatchesResource, DeletedBatchKind,
    DeletedMessageBatch, RequestCounts,
};
