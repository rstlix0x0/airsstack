//! [`HttpTransport`] trait — user-extension seam for HTTP transports.
//!
//! Pure trait definition — no inline tests per the unit-test-mandate
//! exemption #3 (the trait body has no executable logic; the doctest below
//! is the compile check).
//!
//! Responsibilities:
//! - Define [`HttpTransport`], the single send-one-request seam the SDK
//!   client consumes via generics throughout the codebase.
//!
//! Not responsible for:
//! - Status-code interpretation — implementations return `Ok` for any
//!   completed HTTP exchange; the layer above maps non-2xx to API errors.

use bytes::Bytes;
use http::{Request, Response};

use super::BodyStream;
use crate::error::TransportError;

/// HTTP transport boundary.
///
/// Implementations send a single HTTP request and surface the response with
/// its body as an incremental [`BodyStream`]. Wire-layer failures return
/// [`TransportError`]; HTTP 4xx/5xx responses are NOT errors at this layer.
///
/// # Examples
///
/// ```no_run
/// use async_trait::async_trait;
/// use bytes::Bytes;
/// use http::{Request, Response};
/// use openrouter_rs::error::TransportError;
/// use openrouter_rs::transport::{BodyStream, HttpTransport};
///
/// struct NoopTransport;
///
/// #[async_trait]
/// impl HttpTransport for NoopTransport {
///     async fn send(&self, _req: Request<Bytes>) -> Result<Response<BodyStream>, TransportError> {
///         Err(TransportError::Other("noop".into()))
///     }
/// }
/// ```
//
// dyn: async-fn-in-trait via `async-trait` while dyn-compatible
// async-fn-in-trait is not yet ergonomic; the trait is also a public
// user-extension seam, so downstream callers may need to erase the type.
#[async_trait::async_trait]
pub trait HttpTransport: Send + Sync + 'static {
    /// Send a request and return the response with a streamed body.
    ///
    /// # Errors
    /// Returns a [`TransportError`] when the transport fails to dispatch the
    /// request, complete the TLS handshake, or surface the response headers.
    /// HTTP-level non-2xx responses are NOT errors at this layer.
    async fn send(&self, req: Request<Bytes>) -> Result<Response<BodyStream>, TransportError>;
}
