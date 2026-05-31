//! Incremental response body stream type returned by every [`HttpTransport`].
//!
//! Pure type alias — no inline tests per the unit-test-mandate exemption #2.
//! Isolated in its own file because the alias is a `Pin<Box<dyn Stream<…>>>`,
//! one of the few trait-object sites in the SDK; the justification sits next
//! to the only place callers see the trait object.
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
/// Each item yields a chunk of the response body, or a [`TransportError`] if
/// the stream is interrupted mid-flight.
///
/// # Examples
///
/// ```no_run
/// use openrouter_rs::transport::BodyStream;
/// fn takes_stream(_s: BodyStream) {
///     // obtained from HttpTransport::send
/// }
/// ```
pub type BodyStream = Pin<Box<dyn Stream<Item = Result<Bytes, TransportError>> + Send + 'static>>;
