//! Default `reqwest`-backed implementer of [`crate::Transport`] (and, via the
//! blanket impl, [`crate::HttpTransport`]).
//!
//! Sits behind the `transport-reqwest` feature so `reqwest` and its error
//! mapping never compile into builds that disable it.

use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_core::Stream;
use http::{Request, Response};
use pin_project_lite::pin_project;

use crate::BodyStream;
use crate::error::TransportError;
use crate::transport::Transport;

/// Default `reqwest`-backed transport.
///
/// `reqwest::Client` is internally `Arc`-shared, so cloning shares the
/// underlying connection pool.
///
/// # Examples
///
/// ```
/// use airs_transport::ReqwestTransport;
/// let transport = ReqwestTransport::try_new().expect("transport built");
/// ```
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    inner: reqwest::Client,
}

impl ReqwestTransport {
    /// Construct a transport whose `User-Agent` is supplied by the caller.
    ///
    /// Consumer SDKs pass their own branded UA (e.g. `"clauders/0.1.0"`) so
    /// on-wire identification is preserved after the transport moved out of
    /// the SDK crate.
    ///
    /// # Errors
    /// Returns [`TransportError::Build`] when the underlying `reqwest::Client`
    /// cannot initialize (typically a TLS-backend load failure).
    pub fn try_new_with_user_agent(user_agent: &str) -> Result<Self, TransportError> {
        reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .map(|inner| Self { inner })
            .map_err(|e| TransportError::Build(e.to_string()))
    }

    /// Construct a transport with a default `airs-transport/<version>` UA.
    ///
    /// # Errors
    /// Returns [`TransportError::Build`] when the underlying `reqwest::Client`
    /// cannot initialize (typically a TLS-backend load failure).
    pub fn try_new() -> Result<Self, TransportError> {
        Self::try_new_with_user_agent(concat!("airs-transport/", env!("CARGO_PKG_VERSION")))
    }

    /// Construct a transport from a caller-supplied `reqwest::Client`.
    ///
    /// Use this for custom timeouts, proxies, TLS roots, or shared
    /// instrumentation.
    ///
    /// # Examples
    ///
    /// ```
    /// use airs_transport::ReqwestTransport;
    /// let transport = ReqwestTransport::from_client(reqwest::Client::new());
    /// ```
    #[must_use]
    pub const fn from_client(client: reqwest::Client) -> Self {
        Self { inner: client }
    }
}

#[async_trait::async_trait]
impl Transport for ReqwestTransport {
    type Request = Request<Bytes>;
    type Response = Response<BodyStream>;
    type Error = TransportError;

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

/// Pure classification decision over extracted error properties.
///
/// Order matters: timeout, then connect, then TLS, then request-build, then
/// body — because `reqwest`'s `is_request` flag also covers the more-specific
/// categories at the hyper layer.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "private classifier maps one bool per reqwest error flag; enum wrappers would add boilerplate with no caller benefit"
)]
fn classify(
    elapsed: Duration,
    is_timeout: bool,
    is_connect: bool,
    is_tls: bool,
    is_request: bool,
    is_body: bool,
    msg: &str,
) -> TransportError {
    if is_timeout {
        TransportError::Timeout { elapsed }
    } else if is_connect {
        TransportError::Network(msg.to_owned())
    } else if is_tls {
        TransportError::Tls(msg.to_owned())
    } else if is_request {
        TransportError::Build(msg.to_owned())
    } else if is_body {
        TransportError::BodyStream(msg.to_owned())
    } else {
        TransportError::Other(msg.to_owned())
    }
}

/// Detect TLS-related error text. `reqwest` does not expose its TLS error
/// type, so the SDK matches tokens that reliably appear in `rustls`/`webpki`
/// messages.
fn is_tls_message(s: &str) -> bool {
    s.contains("certificate")
        || s.contains("handshake")
        || s.contains("TLS")
        || s.contains("tls ")
        || s.contains("rustls")
        || s.contains("webpki")
}

/// Walk the error source chain, returning true on the first TLS-looking message.
fn is_tls_error_chain(e: &reqwest::Error) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(e);
    while let Some(err) = current {
        if is_tls_message(&err.to_string()) {
            return true;
        }
        current = err.source();
    }
    false
}

/// Map a `reqwest::Error` to a [`TransportError`]. The elapsed time is
/// measured by the caller so the timeout variant carries a real value.
fn classify_reqwest_error(e: &reqwest::Error, elapsed: Duration) -> TransportError {
    classify(
        elapsed,
        e.is_timeout(),
        e.is_connect(),
        is_tls_error_chain(e),
        e.is_request(),
        e.is_body(),
        &e.to_string(),
    )
}

pin_project! {
    /// Adapt a `reqwest` byte stream into a [`BodyStream`], remapping each
    /// `reqwest::Error` to [`TransportError::BodyStream`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_build_a_client() {
        assert!(ReqwestTransport::try_new().is_ok());
        assert!(ReqwestTransport::try_new_with_user_agent("clauders/0.0.0-test").is_ok());
    }

    #[test]
    fn tls_messages_detected() {
        for s in [
            "invalid peer certificate",
            "handshake failed",
            "rustls error",
            "webpki: cert expired",
            "TLS alert",
        ] {
            assert!(is_tls_message(s), "expected TLS detection for {s:?}");
        }
        assert!(!is_tls_message("connection refused"));
        assert!(!is_tls_message("timed out"));
    }

    #[test]
    fn classify_prioritizes_timeout_then_connect_then_tls() {
        let elapsed = Duration::from_secs(3);
        assert!(matches!(
            classify(elapsed, true, true, true, true, true, "x"),
            TransportError::Timeout { .. }
        ));
        assert!(matches!(
            classify(elapsed, false, true, true, true, true, "x"),
            TransportError::Network(_)
        ));
        assert!(matches!(
            classify(elapsed, false, false, true, true, true, "x"),
            TransportError::Tls(_)
        ));
        assert!(matches!(
            classify(elapsed, false, false, false, true, true, "x"),
            TransportError::Build(_)
        ));
        assert!(matches!(
            classify(elapsed, false, false, false, false, true, "x"),
            TransportError::BodyStream(_)
        ));
        assert!(matches!(
            classify(elapsed, false, false, false, false, false, "x"),
            TransportError::Other(_)
        ));
    }
}
