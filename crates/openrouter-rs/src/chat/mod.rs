//! Chat-completion request and response types for `POST /chat/completions`.
//!
//! Exists as a dedicated endpoint module so request-building types, the
//! type-state request builder, and response-decoding types live together and
//! evolve with the endpoint.
//!
//! Responsibilities:
//! - Request: [`ChatRequest`] + [`ChatRequestBuilder`], and the message pieces
//!   [`Message`] / [`Role`] / [`MessageContent`] / [`ContentPart`].
//! - Response: [`ChatCompletion`] / [`Choice`] / [`ResponseMessage`] /
//!   [`FinishReason`] and [`Usage`].
//!
//! Not responsible for sending requests — the resource/transport layer dispatches
//! a built [`ChatRequest`].

pub mod builder;
pub mod message;
pub mod request;
pub mod resource;
pub mod response;
pub mod usage;

#[cfg(feature = "streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
pub mod stream;
#[cfg(feature = "streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
pub mod stream_chunk;

pub use builder::{ChatRequestBuilder, FieldState, Missing, Present};
pub use message::{ContentPart, Message, MessageContent, Role};
pub use request::ChatRequest;
pub use resource::ChatResource;
pub use response::{ChatCompletion, Choice, FinishReason, ResponseMessage};
pub use usage::Usage;

#[cfg(feature = "streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
pub use stream::ChatStream;
#[cfg(feature = "streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
pub use stream_chunk::{ChunkChoice, ChunkDelta, StreamChunk};
