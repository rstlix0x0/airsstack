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
pub mod response;
pub mod usage;

pub use builder::{ChatRequestBuilder, FieldState, Missing, Present};
pub use message::{ContentPart, Message, MessageContent, Role};
pub use request::ChatRequest;
pub use response::{ChatCompletion, Choice, FinishReason, ResponseMessage};
pub use usage::Usage;
