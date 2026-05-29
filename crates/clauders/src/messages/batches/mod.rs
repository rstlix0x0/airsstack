//! Message Batches API surface.
//!
//! Exists as a feature-gated submodule of `messages` so batch-related
//! types are only compiled when the `messages-batches` Cargo feature is
//! enabled. The batches surface is separate from the core Messages API
//! because most callers never submit batch workloads.
//!
//! Responsibilities:
//! - Re-export [`BatchesResource`] as the primary entry point, reached
//!   via `client.messages().batches()`.
//! - Re-export all public wire types — [`BatchRequest`], [`Batch`],
//!   [`BatchResult`], [`DeletedMessageBatch`], and so on — so callers
//!   import from `clauders::messages::*` without navigating sub-modules.
//!
//! Not responsible for:
//! - HTTP transport — that lives in [`crate::transport`].
//! - Client construction — that is the builder's job.
//!
//! Entry point: [`BatchesResource`], obtained via
//! `client.messages().batches()`.

pub mod resource;
pub mod results;
pub mod types;

#[doc(inline)]
pub use resource::BatchesResource;
#[doc(inline)]
pub use results::BatchResultStream;
#[doc(inline)]
pub use types::{
    Batch, BatchKind, BatchList, BatchRequest, BatchRequestBuilder, BatchResult, BatchResultRow,
    BatchStatus, BatchedMessageRequest, DeletedBatchKind, DeletedMessageBatch, RequestCounts,
};
