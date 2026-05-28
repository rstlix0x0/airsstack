//! Incremental response body stream type returned by every [`HttpTransport`].
//!
//! Exists as its own module because the type is a `Pin<Box<dyn Stream<…>>>`
//! — one of the small number of trait-object sites in the SDK. Isolating it
//! puts the justification next to the only place callers see the trait
//! object, instead of burying it inside a larger file.
//!
//! Responsibilities:
//! - Define the [`BodyStream`] type alias that every [`HttpTransport`]
//!   implementation produces and every body-consuming layer accepts.
//!
//! [`HttpTransport`]: super::HttpTransport

use bytes::Bytes;
use futures_core::Stream;
use std::pin::Pin;

use crate::error::TransportError;

// dyn: heterogeneous concrete body-stream types across transport
// implementations are stored uniformly behind this alias.
/// Incremental HTTP response body stream.
///
/// Each item yields a chunk of the response body or a [`TransportError`]
/// if the stream is interrupted mid-flight.
pub type BodyStream =
    Pin<Box<dyn Stream<Item = Result<Bytes, TransportError>> + Send + 'static>>;
