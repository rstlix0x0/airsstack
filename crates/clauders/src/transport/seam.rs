//! [`HttpTransport`] trait — user-extension seam for HTTP transports.
//!
//! Exists as its own module so the trait stands alone in the file the
//! `async-trait` macro and user-supplied implementations refer to.
//! Downstream readers can open one short file to learn the contract before
//! tracing into concrete implementations.
//!
//! Responsibilities:
//! - Define [`HttpTransport`], the single send-one-request seam consumed
//!   by the SDK client via generics throughout the codebase.
//!
//! Not responsible for:
//! - Status-code interpretation — implementations return `Ok` for any
//!   completed HTTP exchange regardless of status; the layer above maps
//!   non-2xx into [`crate::error::ApiError`].

use bytes::Bytes;
use http::{Request, Response};

use super::BodyStream;
use crate::error::TransportError;

/// HTTP transport boundary.
///
/// Implementations send a single HTTP request and surface the response with
/// its body as an incremental [`BodyStream`]. Errors from the wire layer are
/// returned as [`TransportError`]; HTTP-level 4xx/5xx responses are NOT
/// errors at this layer — the layer above interprets status codes.
///
/// # Examples
///
/// Custom transport implementations look like:
///
/// ```no_run
/// use async_trait::async_trait;
/// use bytes::Bytes;
/// use http::{Request, Response};
/// use clauders::error::TransportError;
/// use clauders::transport::{BodyStream, HttpTransport};
///
/// struct NoopTransport;
///
/// #[async_trait]
/// impl HttpTransport for NoopTransport {
///     async fn send(
///         &self,
///         _req: Request<Bytes>,
///     ) -> Result<Response<BodyStream>, TransportError> {
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
    /// Returns a [`TransportError`] when the underlying transport fails to
    /// dispatch the request, complete the TLS handshake, or surface the
    /// response headers. HTTP-level non-2xx responses are NOT errors at this
    /// layer.
    async fn send(&self, req: Request<Bytes>) -> Result<Response<BodyStream>, TransportError>;
}
