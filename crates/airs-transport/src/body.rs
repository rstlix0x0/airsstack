//! Incremental response body stream type produced by the HTTP transport.
//!
//! Pure type alias — no inline tests per the unit-test-mandate exemption #2.
//! Isolated because the alias is a `Pin<Box<dyn Stream<…>>>`, one of the few
//! trait-object sites; the justification sits next to it.

use bytes::Bytes;
use futures_core::Stream;
use std::pin::Pin;

use crate::error::TransportError;

// dyn: heterogeneous concrete body-stream types are stored uniformly here.
/// Incremental HTTP response body stream.
///
/// Each item yields a chunk of the response body, or a [`TransportError`] if
/// the stream is interrupted mid-flight.
///
/// # Examples
///
/// ```no_run
/// use airs_transport::BodyStream;
/// fn takes_stream(_s: BodyStream) {
///     // obtained from HttpTransport::send
/// }
/// ```
pub type BodyStream = Pin<Box<dyn Stream<Item = Result<Bytes, TransportError>> + Send + 'static>>;
