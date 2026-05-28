//! Default `reqwest`-backed [`HttpTransport`] implementation.
//!
//! Exists as its own module so the concrete transport — together with the
//! `reqwest`-specific error mapping it owns — sits behind the
//! `transport-reqwest` Cargo feature and never compiles into builds that
//! disable that feature. Keeping the implementation in one file makes the
//! `reqwest::Client` ↔ [`HttpTransport`] adapter trivial to audit.
//!
//! Responsibilities:
//! - Define [`ReqwestTransport`], the default [`HttpTransport`] backed by
//!   a shared `reqwest::Client`.
//! - Map `reqwest::Error` cases into [`TransportError`] categories the
//!   SDK's retry layer can act on, attaching wallclock elapsed time to
//!   the timeout variant.
//! - Re-pin and re-type `reqwest::Response::bytes_stream()` into the
//!   crate-wide [`BodyStream`] alias.
//!
//! Not responsible for:
//! - Status-code interpretation — non-2xx responses are returned as `Ok`.
//! - Retry, backoff, or rate-limit handling.

use std::time::Instant;

use bytes::Bytes;
use futures_core::Stream;
use http::{Request, Response};

use super::{BodyStream, HttpTransport};
use crate::error::TransportError;

/// Default `reqwest`-backed transport.
///
/// `reqwest::Client` is internally `Arc`-shared, so cloning this struct
/// shares the underlying connection pool.
///
/// # Examples
///
/// ```
/// use clauders::transport::ReqwestTransport;
///
/// let transport = ReqwestTransport::new();
/// ```
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    inner: reqwest::Client,
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self {
            inner: reqwest::Client::builder()
                .user_agent(concat!("clauders/", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl ReqwestTransport {
    /// Construct a new transport with default settings.
    ///
    /// The underlying `reqwest::Client` is configured with a
    /// `User-Agent` header identifying the SDK version.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::transport::ReqwestTransport;
    ///
    /// let transport = ReqwestTransport::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a transport from a caller-supplied `reqwest::Client`.
    ///
    /// Use this when the caller needs custom timeouts, proxies, TLS
    /// roots, or shared instrumentation across multiple SDK clients.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::transport::ReqwestTransport;
    ///
    /// let client = reqwest::Client::new();
    /// let transport = ReqwestTransport::from_client(client);
    /// ```
    #[must_use]
    pub const fn from_client(client: reqwest::Client) -> Self {
        Self { inner: client }
    }
}

#[async_trait::async_trait]
impl HttpTransport for ReqwestTransport {
    async fn send(&self, req: Request<Bytes>) -> Result<Response<BodyStream>, TransportError> {
        let (parts, body) = req.into_parts();
        let url = parts.uri.to_string();

        let mut rb = self.inner.request(parts.method.clone(), &url);
        for (k, v) in &parts.headers {
            rb = rb.header(k, v);
        }
        rb = rb.body(body);

        let started = Instant::now();
        let resp = rb
            .send()
            .await
            .map_err(|e| classify_reqwest_error(&e, started.elapsed()))?;

        let status = resp.status();
        let version = resp.version();
        let headers = resp.headers().clone();

        let byte_stream = resp.bytes_stream();
        let mapped: BodyStream = Box::pin(into_typed_stream(byte_stream));

        let mut out = Response::new(mapped);
        *out.status_mut() = status;
        *out.version_mut() = version;
        *out.headers_mut() = headers;

        Ok(out)
    }
}

/// Map a `reqwest` error to a [`TransportError`] variant.
///
/// The elapsed time is measured by the caller from just before the
/// `send()` call so the `Timeout` variant carries a real wallclock value
/// rather than a zero placeholder.
fn classify_reqwest_error(e: &reqwest::Error, elapsed: std::time::Duration) -> TransportError {
    if e.is_timeout() {
        return TransportError::Timeout { elapsed };
    }
    if e.is_connect() {
        return TransportError::Network(e.to_string());
    }
    if e.is_request() {
        return TransportError::Build(e.to_string());
    }
    if e.is_body() {
        return TransportError::BodyStream(e.to_string());
    }
    TransportError::Other(e.to_string())
}

/// Adapt a `reqwest` byte stream into a [`BodyStream`].
///
/// Each `reqwest::Error` from the body stream maps to
/// [`TransportError::BodyStream`] so callers see a uniform error type.
fn into_typed_stream<S>(s: S) -> impl Stream<Item = Result<Bytes, TransportError>> + Send + 'static
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct Wrap<S>(S);

    impl<S> Stream for Wrap<S>
    where
        S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
    {
        type Item = Result<Bytes, TransportError>;
        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match Pin::new(&mut self.0).poll_next(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Ready(Some(Ok(b))) => Poll::Ready(Some(Ok(b))),
                Poll::Ready(Some(Err(e))) => {
                    Poll::Ready(Some(Err(TransportError::BodyStream(e.to_string()))))
                }
            }
        }
    }

    Wrap(Box::pin(s))
}
