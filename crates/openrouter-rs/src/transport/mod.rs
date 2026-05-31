//! HTTP transport boundary the SDK sends every request through.
//!
//! Exists as its own module so the wire-level send seam sits behind a Cargo
//! feature (`transport-reqwest`), so tests swap a mock implementation at
//! compile time without paying for dynamic dispatch per request, and so the
//! trait and each concrete implementation live in their own file.
//!
//! Responsibilities:
//! - Define [`HttpTransport`] (the user-extension seam) and [`BodyStream`]
//!   (the incremental response body type every implementation returns).
//! - Ship the default [`ReqwestTransport`] behind `transport-reqwest`.
//! - Provide the `mockall`-generated `MockHttpTransport` behind the private
//!   `__test-mocks` feature.
//!
//! Not responsible for:
//! - Interpreting HTTP status codes — 4xx/5xx responses surface as `Ok`; the
//!   layer above maps them to API errors.
//! - Retry, backoff, or rate-limit handling.
//!
//! Entry point: [`HttpTransport::send`].

pub mod body;
pub mod seam;

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub mod reqwest_impl;

#[cfg(feature = "__test-mocks")]
#[cfg_attr(docsrs, doc(cfg(feature = "__test-mocks")))]
pub mod mock;

#[doc(inline)]
pub use body::BodyStream;
#[doc(inline)]
pub use seam::HttpTransport;

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
#[doc(inline)]
pub use reqwest_impl::ReqwestTransport;

#[cfg(feature = "__test-mocks")]
#[cfg_attr(docsrs, doc(cfg(feature = "__test-mocks")))]
pub use mock::MockHttpTransport;
