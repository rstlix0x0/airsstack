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
//!   the timeout variant and inspecting the error source chain so TLS
//!   failures surface distinctly from generic request-build failures.
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
use pin_project_lite::pin_project;

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
/// let transport = ReqwestTransport::try_new().expect("transport built");
/// ```
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    inner: reqwest::Client,
}

impl ReqwestTransport {
    /// Construct a new transport with default settings.
    ///
    /// The underlying `reqwest::Client` is configured with a
    /// `User-Agent` header identifying the SDK version.
    ///
    /// # Errors
    /// Returns [`TransportError::Build`] when the underlying
    /// `reqwest::Client` cannot initialize — typically because the
    /// platform TLS backend failed to load.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::transport::ReqwestTransport;
    ///
    /// let transport = ReqwestTransport::try_new().expect("transport built");
    /// ```
    pub fn try_new() -> Result<Self, TransportError> {
        reqwest::Client::builder()
            .user_agent(concat!("clauders/", env!("CARGO_PKG_VERSION")))
            .build()
            .map(|inner| Self { inner })
            .map_err(|e| TransportError::Build(e.to_string()))
    }

    /// Construct a transport from a caller-supplied `reqwest::Client`.
    ///
    /// Use this when the caller needs custom timeouts, proxies, TLS
    /// roots, or shared instrumentation across multiple SDK clients.
    ///
    /// The `reqwest::Client` type is part of the public contract when
    /// the `transport-reqwest` feature is enabled — consumers enabling
    /// this feature are expected to depend on `reqwest` themselves and
    /// pin a compatible major version.
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

        let mapped: BodyStream = Box::pin(BodyStreamAdapter::new(resp.bytes_stream()));

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
///
/// Order matters: more-specific classifiers must come first because
/// `reqwest::Error::is_request` matches the catch-all `Kind::Request`
/// wrapper that also covers timeouts, connect failures, and TLS errors
/// at the underlying hyper layer. The TLS check walks the source chain
/// using string heuristics on the chained error messages — `reqwest`
/// does not expose the concrete TLS error type, so a precise downcast
/// would require depending on the rustls crate directly.
fn classify_reqwest_error(e: &reqwest::Error, elapsed: std::time::Duration) -> TransportError {
    if e.is_timeout() {
        return TransportError::Timeout { elapsed };
    }
    if e.is_connect() {
        return TransportError::Network(e.to_string());
    }
    if is_tls_error_chain(e) {
        return TransportError::Tls(e.to_string());
    }
    if e.is_request() {
        return TransportError::Build(e.to_string());
    }
    if e.is_body() {
        return TransportError::BodyStream(e.to_string());
    }
    TransportError::Other(e.to_string())
}

/// Heuristic detection of TLS errors by walking the source chain.
///
/// Inspects each `Display` representation in the chain for tokens that
/// reliably appear in `rustls` / `webpki` / `hyper-rustls` error
/// messages. Returns true on the first match.
fn is_tls_error_chain(e: &reqwest::Error) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(e);
    while let Some(err) = current {
        let s = err.to_string();
        if s.contains("certificate")
            || s.contains("handshake")
            || s.contains("TLS")
            || s.contains("tls ")
            || s.contains("rustls")
            || s.contains("webpki")
            || s.contains("invalid peer certificate")
        {
            return true;
        }
        current = err.source();
    }
    false
}

pin_project! {
    /// Adapt a `reqwest` byte stream into a [`BodyStream`] by remapping
    /// each `reqwest::Error` to [`TransportError::BodyStream`] so callers
    /// see a uniform error type. `pin_project_lite` lets the inner stream
    /// stay un-pinned at construction time; only the outer adapter is
    /// boxed by the caller.
    struct BodyStreamAdapter<S> {
        #[pin]
        inner: S,
    }
}

impl<S> BodyStreamAdapter<S> {
    const fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Stream for BodyStreamAdapter<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>>,
{
    type Item = Result<Bytes, TransportError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(b))) => Poll::Ready(Some(Ok(b))),
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(TransportError::BodyStream(e.to_string()))))
            }
        }
    }
}
