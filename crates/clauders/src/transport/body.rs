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
///
/// # Examples
///
/// `BodyStream` is the response body type returned by every
/// [`HttpTransport::send`](super::HttpTransport::send) call. Drain it
/// with `futures_util::StreamExt::next()` or `futures::TryStreamExt`:
///
/// ```no_run
/// use clauders::transport::BodyStream;
///
/// fn takes_stream(_s: BodyStream) {
///     // dispatch through HttpTransport::send to obtain one
/// }
/// ```
pub type BodyStream = Pin<Box<dyn Stream<Item = Result<Bytes, TransportError>> + Send + 'static>>;
