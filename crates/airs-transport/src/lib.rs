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
