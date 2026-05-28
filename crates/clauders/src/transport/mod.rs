//! HTTP transport boundary the SDK sends every Anthropic request through.
//!
//! Exists as its own module so the wire-level send seam can sit behind a
//! Cargo feature (`transport-reqwest`), so tests can swap a mock
//! implementation at compile time without paying for dynamic dispatch on
//! every request, and so the trait and each concrete implementation live in
//! their own file.
//!
//! Responsibilities:
//! - Define [`HttpTransport`] (the user-extension seam) and [`BodyStream`]
//!   (the incremental response body type every implementation returns).
//! - Provide the `mockall`-generated [`MockHttpTransport`] behind the
//!   private `__test-mocks` feature for downstream test code.
//!
//! Not responsible for:
//! - Interpreting HTTP status codes — 4xx/5xx responses surface as `Ok`;
//!   the layer above maps them to API errors.
//! - Retry, backoff, rate-limit handling, or request signing — those live
//!   in the client layer.
//!
//! Entry point: [`HttpTransport::send`].

pub mod body;
pub mod seam;

#[cfg(feature = "__test-mocks")]
#[cfg_attr(docsrs, doc(cfg(feature = "__test-mocks")))]
pub mod mock;

#[doc(inline)]
pub use body::BodyStream;
#[doc(inline)]
pub use seam::HttpTransport;

#[cfg(feature = "__test-mocks")]
#[cfg_attr(docsrs, doc(cfg(feature = "__test-mocks")))]
pub use mock::MockHttpTransport;
