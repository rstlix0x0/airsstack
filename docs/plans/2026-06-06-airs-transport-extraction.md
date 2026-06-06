# airs-transport Extraction Implementation Plan

**Goal:** Centralize the workspace's transport substrate into a single `airs-transport` crate — a generic `Transport` contract with an `HttpTransport`/`reqwest` layer on top — that `clauders` and `openrouter-rs` both consume.

**Architecture:** A new leaf crate `crates/airs-transport` owns a generic `Transport` trait (associated `Request`/`Response`/`Error`, one `async fn send`), an `HttpTransport` marker sub-trait blanket-implemented for any `Transport` fixed to the HTTP types, the `reqwest`-backed concrete transport, the `mockall` fake, `TransportError`, and `collect_body`. The two SDK crates delete their local copies and re-export the new crate as their `transport` module, keeping every existing public path. `reqwest`/`mockall` stay behind `transport-reqwest` / `__test-mocks`, which the consumers forward.

**Tech Stack:** Rust 2024 edition, `async-trait`, associated-type traits with a blanket impl, `reqwest` (rustls), `mockall`, `pin-project-lite`, `thiserror`, `tokio` (dev), `cargo hack`.

---

## File map

```
crates/airs-transport/Cargo.toml             — [create] manifest, feature gates, deps
crates/airs-transport/README.md              — [create] crate description + boundary test
crates/airs-transport/src/lib.rs             — [create] module decls, re-exports, crate docs
crates/airs-transport/src/error.rs           — [create] TransportError + is_retryable
crates/airs-transport/src/transport.rs       — [create] generic Transport trait
crates/airs-transport/src/http_transport.rs  — [create] HttpTransport marker + blanket impl
crates/airs-transport/src/body.rs            — [create] BodyStream alias
crates/airs-transport/src/collect.rs         — [create] collect_body + MAX_RESPONSE_BODY_BYTES
crates/airs-transport/src/reqwest_impl.rs    — [create] ReqwestTransport (impl Transport) + classifier + UA seam
crates/airs-transport/src/mock.rs            — [create] MockHttpTransport (mocks Transport)
Cargo.toml (root)                            — [modify] register airs-transport member

crates/clauders/Cargo.toml                   — [modify] add dep, forward features, drop reqwest
crates/clauders/src/lib.rs                   — [modify] transport module → re-export shim
crates/clauders/src/error.rs                 — [modify] delete TransportError, re-export it
crates/clauders/src/wire_helpers.rs          — [modify] import collect_body/MAX
crates/clauders/src/client.rs                — [modify] UA call + placeholder impl Transport
crates/clauders/src/messages/resource.rs     — [modify] import Transport for .send()
crates/clauders/src/models/resource.rs       — [modify] import Transport for .send()
crates/clauders/src/messages/batches/resource.rs — [modify] import Transport for .send()
crates/clauders/src/transport/               — [delete] whole directory

crates/openrouter-rs/Cargo.toml              — [modify] add dep, forward features, drop reqwest
crates/openrouter-rs/src/lib.rs              — [modify] transport module → re-export shim
crates/openrouter-rs/src/error.rs            — [modify] delete TransportError, re-export it
crates/openrouter-rs/src/wire_helpers.rs     — [modify] import collect_body/MAX
crates/openrouter-rs/src/client.rs           — [modify] UA call + placeholder impl Transport
crates/openrouter-rs/src/chat/resource.rs    — [modify] import Transport for .send()
crates/openrouter-rs/src/models/resource.rs  — [modify] import Transport for .send()
crates/openrouter-rs/src/transport/          — [delete] whole directory
```

Task → file assignment: T1 = manifest/README/lib/error + member. T2 = transport/http_transport/body/collect. T3 = reqwest_impl/mock + UA seam. T4 = `clauders` cutover. T5 = `openrouter-rs` cutover + final gate.

---

### Task 1 — Scaffold `airs-transport` with `TransportError`

**Files:**
- Create `crates/airs-transport/Cargo.toml`
- Create `crates/airs-transport/README.md`
- Create `crates/airs-transport/src/lib.rs`
- Create `crates/airs-transport/src/error.rs`
- Modify root `Cargo.toml`

**Steps:**

1. Create `crates/airs-transport/src/error.rs` with the enum **and its tests**:

   ```rust
   //! Transport-layer error type returned by the HTTP specialization of
   //! [`crate::Transport`].
   //!
   //! Generic over provider: wire-level failure categories (network, TLS,
   //! timeout, body framing, request build) with no API-specific meaning.

   use std::time::Duration;

   /// Failures originating in the HTTP transport layer.
   ///
   /// Each variant maps to a failure category the SDK distinguishes without
   /// inspecting message strings. Use [`TransportError::is_retryable`] to decide
   /// whether a request can be safely re-issued with the same body.
   ///
   /// # Examples
   ///
   /// ```
   /// use airs_transport::TransportError;
   /// use std::time::Duration;
   ///
   /// assert!(TransportError::Network("connection refused".into()).is_retryable());
   /// assert!(!TransportError::Tls("bad certificate".into()).is_retryable());
   /// assert!(TransportError::Timeout { elapsed: Duration::from_secs(30) }.is_retryable());
   /// ```
   #[derive(Debug, thiserror::Error)]
   #[non_exhaustive]
   pub enum TransportError {
       /// Network-level failure (connection refused, reset, DNS, etc.).
       #[error("network failure: {0}")]
       Network(String),

       /// TLS handshake or certificate validation failure.
       #[error("TLS error: {0}")]
       Tls(String),

       /// Request did not complete within the configured timeout.
       #[error("request timed out after {elapsed:?}")]
       Timeout {
           /// How long the request was in flight before being aborted.
           elapsed: Duration,
       },

       /// Failure consuming the response body stream after headers arrived.
       #[error("response body stream error: {0}")]
       BodyStream(String),

       /// Failure constructing the outgoing request (URL parse, header value, etc.).
       #[error("request build failed: {0}")]
       Build(String),

       /// Transport failure the SDK cannot categorize more specifically.
       ///
       /// Treated as non-retryable: without a known category the SDK
       /// cannot prove a retry is safe.
       #[error("transport error: {0}")]
       Other(String),
   }

   impl TransportError {
       /// Whether the failure is safe to retry with the same request body.
       ///
       /// Retryable: `Network`, `Timeout` (transient connectivity). All other
       /// variants indicate a request-shape or configuration issue retrying
       /// will not resolve.
       #[must_use]
       pub const fn is_retryable(&self) -> bool {
           matches!(self, Self::Network(_) | Self::Timeout { .. })
       }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn retryable_categories() {
           assert!(TransportError::Network(String::new()).is_retryable());
           assert!(
               TransportError::Timeout {
                   elapsed: Duration::from_secs(1)
               }
               .is_retryable()
           );
           assert!(!TransportError::Tls(String::new()).is_retryable());
           assert!(!TransportError::BodyStream(String::new()).is_retryable());
           assert!(!TransportError::Build(String::new()).is_retryable());
           assert!(!TransportError::Other(String::new()).is_retryable());
       }
   }
   ```

2. Create `crates/airs-transport/src/lib.rs` (declares only `error` for now so it builds):

   ```rust
   //! Generic async transport substrate shared by airsstack SDK crates.
   //!
   //! Layered: [`Transport`] is the generic send-one-request contract;
   //! [`HttpTransport`] is the HTTP specialization (a `Transport` whose
   //! associated types are the `http` crate types); `ReqwestTransport` is the
   //! concrete implementer behind the `transport-reqwest` feature.
   //!
   //! Boundary test for what belongs here: *does the code name a provider, an
   //! endpoint, an API-key format, a model catalog, a sampling range, or a wire
   //! error envelope?* If yes, it belongs in a consumer SDK; if no, it is
   //! eligible for this crate.
   #![forbid(unsafe_code)]
   #![cfg_attr(docsrs, feature(doc_cfg))]

   pub mod error;

   pub use error::TransportError;
   ```

3. Create `crates/airs-transport/Cargo.toml`:

   ```toml
   [package]
   name        = "airs-transport"
   description = "Generic async transport substrate (with HTTP/reqwest layer) shared by airsstack SDK crates."
   version     = "0.1.0"
   edition.workspace      = true
   rust-version.workspace = true
   license.workspace      = true
   repository.workspace   = true
   authors.workspace      = true
   publish.workspace      = true
   readme      = "README.md"
   keywords    = ["http", "transport", "reqwest", "async", "sdk"]
   categories  = ["web-programming::http-client", "asynchronous"]

   [lints]
   workspace = true

   [features]
   default           = []
   transport-reqwest = ["dep:reqwest"]
   __test-mocks      = ["dep:mockall"]

   [dependencies]
   http             = { workspace = true }
   bytes            = { workspace = true }
   futures-core     = { workspace = true }
   pin-project-lite = { workspace = true }
   async-trait      = { workspace = true }
   thiserror        = { workspace = true }
   reqwest          = { workspace = true, optional = true }
   mockall          = { workspace = true, optional = true }

   [dev-dependencies]
   tokio        = { workspace = true, features = ["full", "test-util"] }
   futures-util = { workspace = true }

   [package.metadata.docs.rs]
   all-features = true
   rustdoc-args = ["--cfg", "docsrs"]
   ```

4. Create `crates/airs-transport/README.md`:

   ```markdown
   # airs-transport

   Generic async transport substrate shared by the airsstack SDK crates
   (`clauders`, `openrouter-rs`).

   Layered:

   - `Transport` — the generic send-one-request contract (associated
     `Request`/`Response`/`Error`). Names no HTTP concept.
   - `HttpTransport` — a `Transport` fixed to the HTTP types; a marker sub-trait
     with a blanket impl.
   - `ReqwestTransport` — the concrete HTTP implementer (feature `transport-reqwest`).

   It carries **zero domain knowledge**. Boundary test: *does the code name a
   provider, an endpoint, an API-key format, a model catalog, a sampling range,
   or a wire error envelope?* If yes, it belongs in a consumer SDK; if no, it is
   eligible here.

   ## Public surface

   - `Transport`, `HttpTransport` — the trait layers.
   - `BodyStream` — incremental response-body stream type.
   - `TransportError` — wire-level failure categories with `is_retryable`.
   - `collect_body` / `MAX_RESPONSE_BODY_BYTES` — drain a body stream with a cap.
   - `ReqwestTransport` (feature `transport-reqwest`) — default backend.
   - `MockHttpTransport` (feature `__test-mocks`) — `mockall` fake.
   ```

5. Register the member in root `Cargo.toml`:

   ```toml
   members  = ["crates/airs-transport", "crates/clauders", "crates/openrouter-rs"]
   ```

6. Run build + tests; confirm green:

   ```
   $ cargo test -p airs-transport
   test error::tests::retryable_categories ... ok
   test result: ok. 1 passed; 0 failed

   $ cargo test -p airs-transport --doc
   test result: ok. 1 passed; 0 failed
   ```

7. Commit:

   ```
   feat(airs-transport): scaffold crate with TransportError and workspace member
   ```

---

### Task 2 — Add the `Transport` contract, `HttpTransport` layer, `BodyStream`, and `collect_body`

**Files:**
- Create `crates/airs-transport/src/body.rs`
- Create `crates/airs-transport/src/transport.rs`
- Create `crates/airs-transport/src/http_transport.rs`
- Create `crates/airs-transport/src/collect.rs`
- Modify `crates/airs-transport/src/lib.rs`

**Steps:**

1. Create `crates/airs-transport/src/body.rs`:

   ```rust
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
   ```

2. Create `crates/airs-transport/src/transport.rs` — the generic contract:

   ```rust
   //! The generic [`Transport`] contract: send one request, get one response.
   //!
   //! Names no HTTP concept. The HTTP specialization is [`crate::HttpTransport`];
   //! a non-HTTP transport implements `Transport` with its own associated types.
   //!
   //! Pure trait definition — no inline tests per the unit-test-mandate
   //! exemption #3 (the trait body has no executable logic).

   /// Send-one-request transport boundary.
   ///
   /// Implementations carry no shared interpretation of the request or response
   /// beyond moving one to produce the other. The HTTP specialization fixes the
   /// associated types to the `http` crate types; see [`crate::HttpTransport`].
   #[async_trait::async_trait]
   pub trait Transport: Send + Sync + 'static {
       /// Request the transport accepts.
       type Request: Send;
       /// Response the transport produces on success.
       type Response: Send;
       /// Error the transport produces on failure.
       type Error: Send;

       /// Send a request and return the response.
       ///
       /// # Errors
       /// Returns [`Transport::Error`] when the transport fails to produce a
       /// response. For the HTTP specialization, protocol-level non-success
       /// results (HTTP 4xx/5xx) are NOT errors at this layer.
       async fn send(&self, req: Self::Request) -> Result<Self::Response, Self::Error>;
   }
   ```

3. Create `crates/airs-transport/src/http_transport.rs` — the HTTP layer **and a blanket-impl test**:

   ```rust
   //! [`HttpTransport`] — the HTTP specialization of [`crate::Transport`].
   //!
   //! A marker sub-trait with a blanket impl: any `Transport` whose associated
   //! types are the HTTP types is an `HttpTransport` automatically. SDK clients
   //! bound their generic transport parameter on `HttpTransport`. Named
   //! `http_transport` (not `http`) to avoid shadowing the extern `http` crate.

   use bytes::Bytes;
   use http::{Request, Response};

   use crate::BodyStream;
   use crate::error::TransportError;
   use crate::transport::Transport;

   /// A [`Transport`] specialized to the HTTP request/response/error types.
   ///
   /// This is a marker: it adds no methods. Implement [`Transport`] with the
   /// HTTP associated types and the blanket impl below grants `HttpTransport`.
   /// To call [`Transport::send`] on a value bound by `HttpTransport`, bring
   /// [`Transport`] into scope.
   pub trait HttpTransport:
       Transport<Request = Request<Bytes>, Response = Response<BodyStream>, Error = TransportError>
   {
   }

   impl<T> HttpTransport for T where
       T: Transport<Request = Request<Bytes>, Response = Response<BodyStream>, Error = TransportError>
   {
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       struct Dummy;

       #[async_trait::async_trait]
       impl Transport for Dummy {
           type Request = Request<Bytes>;
           type Response = Response<BodyStream>;
           type Error = TransportError;
           async fn send(
               &self,
               _req: Request<Bytes>,
           ) -> Result<Response<BodyStream>, TransportError> {
               Err(TransportError::Other("dummy".into()))
           }
       }

       fn require_http_transport<T: HttpTransport>() {}

       #[test]
       fn blanket_impl_grants_http_transport() {
           // Compiles only if the blanket impl makes `Dummy: HttpTransport`.
           require_http_transport::<Dummy>();
       }
   }
   ```

4. Create `crates/airs-transport/src/collect.rs` with the drainer **and its tests**:

   ```rust
   //! Drain an incremental [`crate::BodyStream`] into a byte buffer with a cap.
   //!
   //! Generic over provider — operates purely on bytes and a size limit.

   use crate::BodyStream;
   use crate::error::TransportError;

   /// Maximum response body size accepted before truncation.
   ///
   /// 16 MiB is a conservative ceiling well above any plausible non-streaming
   /// API response.
   pub const MAX_RESPONSE_BODY_BYTES: usize = 16 * 1024 * 1024;

   /// Collect a [`BodyStream`] into a byte buffer, stopping at `limit` bytes.
   ///
   /// # Errors
   /// Returns [`TransportError::BodyStream`] if the stream yields an error or if
   /// the accumulated size exceeds `limit`.
   pub async fn collect_body(
       mut stream: BodyStream,
       limit: usize,
   ) -> Result<Vec<u8>, TransportError> {
       let mut buf = Vec::new();
       loop {
           let item = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await;
           match item {
               None => break,
               Some(Err(e)) => return Err(e),
               Some(Ok(chunk)) => {
                   if buf.len() + chunk.len() > limit {
                       return Err(TransportError::BodyStream(format!(
                           "response body exceeded {limit} byte limit"
                       )));
                   }
                   buf.extend_from_slice(&chunk);
               }
           }
       }
       Ok(buf)
   }

   #[cfg(test)]
   mod tests {
       #![expect(
           clippy::unwrap_used,
           reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
       )]

       use super::*;
       use bytes::Bytes;
       use futures_core::Stream;
       use std::pin::Pin;
       use std::task::{Context, Poll};

       fn body_from(payload: &'static [u8]) -> BodyStream {
           struct Once(Option<Bytes>);
           impl Stream for Once {
               type Item = Result<Bytes, TransportError>;
               fn poll_next(
                   mut self: Pin<&mut Self>,
                   _cx: &mut Context<'_>,
               ) -> Poll<Option<Self::Item>> {
                   Poll::Ready(self.0.take().map(Ok))
               }
           }
           Box::pin(Once(Some(Bytes::from_static(payload))))
       }

       #[tokio::test]
       async fn collect_body_drains_within_limit() {
           let bytes = collect_body(body_from(b"hello world"), 1024).await.unwrap();
           assert_eq!(bytes, b"hello world");
       }

       #[tokio::test]
       async fn collect_body_rejects_over_limit() {
           let err = collect_body(body_from(b"too big"), 3).await.unwrap_err();
           assert!(matches!(err, TransportError::BodyStream(_)));
       }
   }
   ```

5. Replace `crates/airs-transport/src/lib.rs` with the module set declared so far:

   ```rust
   //! Generic async transport substrate shared by airsstack SDK crates.
   //!
   //! Layered: [`Transport`] is the generic send-one-request contract;
   //! [`HttpTransport`] is the HTTP specialization (a `Transport` whose
   //! associated types are the `http` crate types); `ReqwestTransport` is the
   //! concrete implementer behind the `transport-reqwest` feature.
   //!
   //! Boundary test for what belongs here: *does the code name a provider, an
   //! endpoint, an API-key format, a model catalog, a sampling range, or a wire
   //! error envelope?* If yes, it belongs in a consumer SDK; if no, it is
   //! eligible for this crate.
   #![forbid(unsafe_code)]
   #![cfg_attr(docsrs, feature(doc_cfg))]

   pub mod body;
   pub mod collect;
   pub mod error;
   pub mod http_transport;
   pub mod transport;

   pub use body::BodyStream;
   pub use collect::{MAX_RESPONSE_BODY_BYTES, collect_body};
   pub use error::TransportError;
   pub use http_transport::HttpTransport;
   pub use transport::Transport;
   ```

6. Run tests; confirm green:

   ```
   $ cargo test -p airs-transport
   test error::tests::retryable_categories ... ok
   test http_transport::tests::blanket_impl_grants_http_transport ... ok
   test collect::tests::collect_body_drains_within_limit ... ok
   test collect::tests::collect_body_rejects_over_limit ... ok
   test result: ok. 4 passed; 0 failed
   ```

7. Commit:

   ```
   feat(airs-transport): add Transport contract, HttpTransport layer, BodyStream, collect_body
   ```

---

### Task 3 — Add `ReqwestTransport` (impl `Transport`, with UA seam) and `MockHttpTransport`

**Files:**
- Create `crates/airs-transport/src/reqwest_impl.rs`
- Create `crates/airs-transport/src/mock.rs`
- Modify `crates/airs-transport/src/lib.rs`

**Steps:**

1. Create `crates/airs-transport/src/reqwest_impl.rs` — implements the generic `Transport` (the blanket impl then grants `HttpTransport`), adopts openrouter's testable classifier, and adds `try_new_with_user_agent`:

   ```rust
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
   ```

2. Create `crates/airs-transport/src/mock.rs` — mocks the generic `Transport` (so `expect_send()` is generated); the blanket impl then makes the mock an `HttpTransport`:

   ```rust
   //! `MockHttpTransport` — `mockall`-generated fake of [`crate::Transport`]
   //! fixed to the HTTP associated types.
   //!
   //! No inline tests per the unit-test-mandate exemption #4 (the body is a
   //! code-generation macro). Gated behind the private `__test-mocks` feature;
   //! production builds never compile this module. The blanket impl in
   //! `http_transport` makes the generated mock an `HttpTransport`.

   use bytes::Bytes;
   use http::{Request, Response};

   use crate::BodyStream;
   use crate::error::TransportError;
   use crate::transport::Transport;

   mockall::mock! {
       /// Mock implementation of [`Transport`] (HTTP types) for tests.
       ///
       /// Set expectations with `expect_send()`; see the `mockall` docs for the
       /// full expectation API.
       pub HttpTransport {}

       #[async_trait::async_trait]
       impl Transport for HttpTransport {
           type Request = Request<Bytes>;
           type Response = Response<BodyStream>;
           type Error = TransportError;
           async fn send(
               &self,
               req: Request<Bytes>,
           ) -> Result<Response<BodyStream>, TransportError>;
       }
   }
   ```

3. Update `crates/airs-transport/src/lib.rs` to declare and re-export the feature-gated modules (final body shown):

   ```rust
   //! Generic async transport substrate shared by airsstack SDK crates.
   //!
   //! Layered: [`Transport`] is the generic send-one-request contract;
   //! [`HttpTransport`] is the HTTP specialization (a `Transport` whose
   //! associated types are the `http` crate types); `ReqwestTransport` is the
   //! concrete implementer behind the `transport-reqwest` feature.
   //!
   //! Boundary test for what belongs here: *does the code name a provider, an
   //! endpoint, an API-key format, a model catalog, a sampling range, or a wire
   //! error envelope?* If yes, it belongs in a consumer SDK; if no, it is
   //! eligible for this crate.
   #![forbid(unsafe_code)]
   #![cfg_attr(docsrs, feature(doc_cfg))]

   pub mod body;
   pub mod collect;
   pub mod error;
   pub mod http_transport;
   pub mod transport;

   #[cfg(feature = "transport-reqwest")]
   #[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
   pub mod reqwest_impl;

   #[cfg(feature = "__test-mocks")]
   #[cfg_attr(docsrs, doc(cfg(feature = "__test-mocks")))]
   pub mod mock;

   pub use body::BodyStream;
   pub use collect::{MAX_RESPONSE_BODY_BYTES, collect_body};
   pub use error::TransportError;
   pub use http_transport::HttpTransport;
   pub use transport::Transport;

   #[cfg(feature = "transport-reqwest")]
   #[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
   pub use reqwest_impl::ReqwestTransport;

   #[cfg(feature = "__test-mocks")]
   #[cfg_attr(docsrs, doc(cfg(feature = "__test-mocks")))]
   pub use mock::MockHttpTransport;
   ```

4. Run the feature-gated tests and matrix; confirm green:

   ```
   $ cargo test -p airs-transport --features transport-reqwest
   test reqwest_impl::tests::constructors_build_a_client ... ok
   test reqwest_impl::tests::tls_messages_detected ... ok
   test reqwest_impl::tests::classify_prioritizes_timeout_then_connect_then_tls ... ok
   test result: ok. 7 passed; 0 failed

   $ cargo build -p airs-transport --features __test-mocks
   Finished

   $ cargo hack --each-feature -p airs-transport
   cargo hack --each-feature finished
   ```

5. Commit:

   ```
   feat(airs-transport): add ReqwestTransport with user-agent seam and MockHttpTransport
   ```

---

### Task 4 — Migrate `clauders` onto `airs-transport`

**Files:**
- Modify `crates/clauders/Cargo.toml`
- Modify `crates/clauders/src/lib.rs`
- Modify `crates/clauders/src/error.rs`
- Modify `crates/clauders/src/wire_helpers.rs`
- Modify `crates/clauders/src/client.rs`
- Modify `crates/clauders/src/messages/resource.rs`
- Modify `crates/clauders/src/models/resource.rs`
- Modify `crates/clauders/src/messages/batches/resource.rs`
- Delete `crates/clauders/src/transport/`

**Steps:**

1. In `crates/clauders/Cargo.toml`, change the two feature definitions and the dependency set:

   ```toml
   # transport-reqwest = ["dep:reqwest"]            # OLD
   transport-reqwest = ["airs-transport/transport-reqwest"]

   # __test-mocks      = ["dep:mockall"]            # OLD
   __test-mocks      = ["airs-transport/__test-mocks"]
   ```

   ```toml
   # add to [dependencies]:
   airs-transport = { path = "../airs-transport" }
   # remove from [dependencies]:
   # reqwest              = { workspace = true, optional = true }
   ```

   Keep `[dev-dependencies] mockall` (used by tests). The build in step 9 confirms no other `reqwest` reference remains.

2. In `crates/clauders/src/lib.rs`, replace the transport module declaration. Change:

   ```rust
   pub mod transport;
   ```

   to:

   ```rust
   #[doc(inline)]
   pub use airs_transport as transport;
   ```

   Leave `pub use error::{ApiError, ApiErrorBody, BuildError, Error, ErrorType, TransportError};` unchanged (preserved by step 3).

3. In `crates/clauders/src/error.rs`, delete the `TransportError` enum, its `impl TransportError { … }`, and its `#[cfg(test)]` cases for `TransportError`. After `use std::time::Duration;` add:

   ```rust
   pub use airs_transport::TransportError;
   ```

   Leave the `Transport(#[from] TransportError)` arm on `Error` unchanged.

4. In `crates/clauders/src/wire_helpers.rs`, delete the local `MAX_RESPONSE_BODY_BYTES` constant and the `collect_body` function (and any `collect_body`-specific test). Keep the existing `use crate::transport::BodyStream;` and add below it:

   ```rust
   use airs_transport::{MAX_RESPONSE_BODY_BYTES, collect_body};
   ```

   Keep `decode_api_error_from_parts`, `decode_error_body`, `parse_retry_after`, `ApiErrorEnvelope`, and their tests.

5. In `crates/clauders/src/client.rs`:

   - Change the import. From:

     ```rust
     use crate::transport::HttpTransport;
     ```

     to:

     ```rust
     use crate::transport::{HttpTransport, Transport};
     ```

   - Convert the no-reqwest placeholder. Replace:

     ```rust
     #[cfg(not(feature = "transport-reqwest"))]
     #[async_trait::async_trait]
     impl HttpTransport for DefaultTransportPlaceholder {
         async fn send(
             &self,
             _req: http::Request<bytes::Bytes>,
         ) -> Result<http::Response<crate::transport::BodyStream>, crate::error::TransportError> {
             Err(crate::error::TransportError::Other(
                 "no transport configured: enable feature `transport-reqwest` or supply a custom transport via the client builder".into(),
             ))
         }
     }
     ```

     with:

     ```rust
     #[cfg(not(feature = "transport-reqwest"))]
     #[async_trait::async_trait]
     impl Transport for DefaultTransportPlaceholder {
         type Request = http::Request<bytes::Bytes>;
         type Response = http::Response<crate::transport::BodyStream>;
         type Error = crate::error::TransportError;
         async fn send(
             &self,
             _req: http::Request<bytes::Bytes>,
         ) -> Result<http::Response<crate::transport::BodyStream>, crate::error::TransportError> {
             Err(crate::error::TransportError::Other(
                 "no transport configured: enable feature `transport-reqwest` or supply a custom transport via the client builder".into(),
             ))
         }
     }
     ```

   - Change the default-transport construction (around line 218). Replace:

     ```rust
     let transport = ReqwestTransport::try_new()
     ```

     with:

     ```rust
     let transport = ReqwestTransport::try_new_with_user_agent(concat!(
         "clauders/",
         env!("CARGO_PKG_VERSION")
     ))
     ```

6. In `crates/clauders/src/messages/resource.rs`, add `Transport` so `self.client.inner.transport.send(http_req)` resolves. Change:

   ```rust
   use crate::transport::{BodyStream, HttpTransport};
   ```

   to:

   ```rust
   use crate::transport::{BodyStream, HttpTransport, Transport};
   ```

7. In `crates/clauders/src/models/resource.rs`, change:

   ```rust
   use crate::transport::{BodyStream, HttpTransport};
   ```

   to:

   ```rust
   use crate::transport::{BodyStream, HttpTransport, Transport};
   ```

8. In `crates/clauders/src/messages/batches/resource.rs`, change:

   ```rust
   use crate::transport::{BodyStream, HttpTransport};
   ```

   to:

   ```rust
   use crate::transport::{BodyStream, HttpTransport, Transport};
   ```

9. Delete the moved directory and run the suite + matrix; confirm green:

   ```
   $ git rm -r crates/clauders/src/transport
   $ cargo test -p clauders
   test result: ok. <N> passed; 0 failed
   $ cargo hack --each-feature -p clauders
   cargo hack --each-feature finished
   ```

10. Commit:

    ```
    refactor(clauders): consume airs-transport for the transport layer
    ```

---

### Task 5 — Migrate `openrouter-rs` onto `airs-transport`

**Files:**
- Modify `crates/openrouter-rs/Cargo.toml`
- Modify `crates/openrouter-rs/src/lib.rs`
- Modify `crates/openrouter-rs/src/error.rs`
- Modify `crates/openrouter-rs/src/wire_helpers.rs`
- Modify `crates/openrouter-rs/src/client.rs`
- Modify `crates/openrouter-rs/src/chat/resource.rs`
- Modify `crates/openrouter-rs/src/models/resource.rs`
- Delete `crates/openrouter-rs/src/transport/`

**Steps:**

1. In `crates/openrouter-rs/Cargo.toml`, change the feature definitions and dependency set:

   ```toml
   # transport-reqwest = ["dep:reqwest"]            # OLD
   transport-reqwest = ["airs-transport/transport-reqwest"]

   # __test-mocks      = ["dep:mockall"]            # OLD
   __test-mocks      = ["airs-transport/__test-mocks"]
   ```

   ```toml
   # add to [dependencies]:
   airs-transport = { path = "../airs-transport" }
   # remove from [dependencies]:
   # reqwest          = { workspace = true, optional = true }
   ```

   Leave `streaming = ["dep:eventsource-stream"]` and `eventsource-stream` untouched. Keep `[dev-dependencies] mockall`.

2. In `crates/openrouter-rs/src/lib.rs`, replace:

   ```rust
   pub mod transport;
   ```

   with:

   ```rust
   #[doc(inline)]
   pub use airs_transport as transport;
   ```

3. In `crates/openrouter-rs/src/error.rs`, delete the `TransportError` enum, its `impl`, and its `TransportError` `#[cfg(test)]` cases. After `use std::time::Duration;` add:

   ```rust
   pub use airs_transport::TransportError;
   ```

   Leave the `Transport(#[from] TransportError)` arm unchanged.

4. In `crates/openrouter-rs/src/wire_helpers.rs`, delete the local `MAX_RESPONSE_BODY_BYTES` constant and the `collect_body` function (and the two `collect_body` tests). Change the import block from:

   ```rust
   use crate::error::{Error, TransportError};
   use crate::headers as h;
   use crate::transport::BodyStream;
   ```

   to:

   ```rust
   use crate::error::{Error, TransportError};
   use crate::headers as h;
   use crate::transport::BodyStream;
   use airs_transport::{MAX_RESPONSE_BODY_BYTES, collect_body};
   ```

   Keep `decode_api_error_from_parts`, `parse_retry_after`, and the envelope structs.

5. In `crates/openrouter-rs/src/client.rs`:

   - Change the import. From:

     ```rust
     use crate::transport::HttpTransport;
     ```

     to:

     ```rust
     use crate::transport::{HttpTransport, Transport};
     ```

   - Convert the no-reqwest placeholder. Replace:

     ```rust
     #[cfg(not(feature = "transport-reqwest"))]
     #[async_trait::async_trait]
     impl HttpTransport for DefaultTransportPlaceholder {
         async fn send(
             &self,
             _req: http::Request<bytes::Bytes>,
         ) -> Result<http::Response<crate::transport::BodyStream>, crate::error::TransportError> {
             Err(crate::error::TransportError::Other(
                 "no transport configured: enable feature `transport-reqwest` or supply a custom transport via the client builder".into(),
             ))
         }
     }
     ```

     with:

     ```rust
     #[cfg(not(feature = "transport-reqwest"))]
     #[async_trait::async_trait]
     impl Transport for DefaultTransportPlaceholder {
         type Request = http::Request<bytes::Bytes>;
         type Response = http::Response<crate::transport::BodyStream>;
         type Error = crate::error::TransportError;
         async fn send(
             &self,
             _req: http::Request<bytes::Bytes>,
         ) -> Result<http::Response<crate::transport::BodyStream>, crate::error::TransportError> {
             Err(crate::error::TransportError::Other(
                 "no transport configured: enable feature `transport-reqwest` or supply a custom transport via the client builder".into(),
             ))
         }
     }
     ```

   - Change the default-transport construction (around line 187). Replace:

     ```rust
     let transport = ReqwestTransport::try_new()
     ```

     with:

     ```rust
     let transport = ReqwestTransport::try_new_with_user_agent(concat!(
         "openrouter-rs/",
         env!("CARGO_PKG_VERSION")
     ))
     ```

6. **Compiler-guided `Transport`-in-scope edits (amended after T4).** The generic resource
   modules (`chat/resource.rs`, `models/resource.rs`) do **not** need a `Transport` import —
   `.send()` on their generic `T: HttpTransport` parameter resolves through the supertrait
   bound. Only code calling `.send()` on a **concrete** transport needs `Transport` in scope,
   which in practice means the integration tests under `crates/openrouter-rs/tests/`. Build
   the crate, and for every file the compiler reports an unresolved `send` method (or an
   unused `HttpTransport` import that should become `Transport`), add `Transport` to that
   file's `use crate::transport::{…}` (swapping out an unused `HttpTransport` where present).
   Do NOT pre-edit the resource modules; only touch the files the compiler points to. Report
   the exact file set you changed.

7. (folded into step 6)

8. Delete the moved directory and run the suite + matrix; confirm green:

   ```
   $ git rm -r crates/openrouter-rs/src/transport
   $ cargo test -p openrouter-rs
   test result: ok. <N> passed; 0 failed
   $ cargo hack --each-feature -p openrouter-rs
   cargo hack --each-feature finished
   ```

9. Run the **full workspace Definition-of-Done gate**; confirm every command green before committing:

   ```
   $ cargo fmt --check
   $ cargo build --workspace
   $ cargo clippy --workspace --all-targets
   $ cargo test --workspace
   $ cargo hack --each-feature -p airs-transport
   $ cargo hack --each-feature -p clauders
   $ cargo hack --each-feature -p openrouter-rs
   ```

10. Commit:

    ```
    refactor(openrouter-rs): consume airs-transport for the transport layer
    ```

---

## Final verification gate

Complete when, from a clean tree, all pass:

- `cargo fmt --check` clean.
- `cargo build --workspace` green.
- `cargo clippy --workspace --all-targets` green.
- `cargo test --workspace` green (unit, integration, doctests, all three crates).
- `cargo hack --each-feature` green for `airs-transport`, `clauders`, `openrouter-rs`.
- `crates/clauders/src/transport/` and `crates/openrouter-rs/src/transport/` no longer exist.
- No `Transport`/`HttpTransport`/`TransportError`/`collect_body` is defined in either consumer; each resolves through `airs-transport`.

---

## Self-review

### Spec-coverage pass

| Spec scope item | Task |
|-----------------|------|
| `Transport` generic trait (§3.1) | T2 |
| `HttpTransport` marker + blanket impl (§3.2) | T2 |
| `ReqwestTransport` impls `Transport` (§3.3) | T3 |
| method named `send` not `call` (§3.1) | T2 (trait), preserved everywhere |
| `BodyStream` alias | T2 |
| `MockHttpTransport` mocks `Transport` | T3 |
| `TransportError` + `is_retryable` | T1 |
| `collect_body` + `MAX_RESPONSE_BODY_BYTES`, unconditional (§6.3) | T2 |
| openrouter's testable `classify` canonical (§6.1) | T3 |
| UA seam `try_new_with_user_agent` (§6.2) | T3 (def), T4/T5 (call sites) |
| slim manifest, `default = []` (§7) | T1 |
| consumer Cargo forwarding + drop reqwest (§8.1) | T4, T5 |
| `lib.rs` re-export shim (§8.2) | T4, T5 |
| `error.rs` re-export, keep `#[from]` (§8.3) | T4, T5 |
| `wire_helpers` import moved items (§8.4) | T4, T5 |
| client UA call + placeholder `impl Transport` (§8.5) | T4, T5 |
| resource modules import `Transport` (§8.6) | T4 (3 files), T5 (2 files) |
| delete moved files (§8.7) | T4, T5 |
| workspace member (with first commit) (§13) | T1 |
| blanket-impl test (§10) | T2 |
| feature matrix, consumer regression, doctests (§10) | T2/T3 own tests; T4/T5 regression; final gate |
| DoD gate (§11) | final gate + T5 step 9 |
| migration order (§12) | T1→T5 ordering |

Every in-scope spec item maps to a task. Out-of-scope items (domain newtypes, config, headers, auth, error envelopes, SSE, API surface) are untouched.

### Placeholder scan

No `TBD`/`TODO`/`implement later`. `<N>`/`<version>` are expected-output / `env!`-interpolated positions, not deferred decisions. Every code block is complete; every command has an expected signal.

### Type-consistency check

- `TransportError` (T1) precedes all references.
- `Transport`, `HttpTransport`, `BodyStream`, `collect_body`, `MAX_RESPONSE_BODY_BYTES` (T2) precede `reqwest_impl`/`mock` (T3) and the consumer edits (T4/T5).
- `ReqwestTransport::try_new_with_user_agent` (T3) precedes its call sites (T4/T5).
- `ReqwestTransport` impls `Transport`; the blanket impl (T2) grants `HttpTransport`, so `Client<T: HttpTransport>` bounds in both consumers are satisfied without change.
- `MockHttpTransport` mocks `Transport` with the HTTP associated types → `expect_send()` is generated (consumer test code unchanged) and the blanket impl makes it an `HttpTransport` (so `Client<MockHttpTransport>` holds).
- The placeholder converts to `impl Transport` (T4/T5 step 5), gaining `HttpTransport` via the blanket; the default type parameter on `Client` still resolves under `not(transport-reqwest)`.
- Resource `.send()` sites get `Transport` in scope (T4/T5) so the supertrait method resolves.
- The re-export `pub use airs_transport as transport;` preserves every `crate::transport::*` path used by the consumers.

No forward references remain.

---

## Execution handoff

Recommended path: **subagent-driven** via `airsstack-sdd:execute-plan` — the change spans many files across three crates and benefits from a fresh coder + reviewer + verifier per task, with the main thread holding the commit gate. Tasks are sequential (each builds on the prior crate state); run T1 → T5 in order, reviewing each receipt before spawning the next.
