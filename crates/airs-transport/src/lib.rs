//! Generic async transport substrate shared by airsstack SDK crates.
//!
//! Layered: [`Transport`] is the generic send-one-request contract;
//! [`HttpTransport`] is the HTTP specialization (a `Transport` whose
//! associated types are the `http` crate types); `ReqwestTransport` is the
//! concrete `reqwest`-backed implementer.
//!
//! Boundary test for what belongs here: *does the code name a provider, an
//! endpoint, an API-key format, a model catalog, a sampling range, or a wire
//! error envelope?* If yes, it belongs in a consumer SDK; if no, it is
//! eligible for this crate.
#![forbid(unsafe_code)]

pub mod body;
pub mod collect;
pub mod error;
pub mod http_transport;
pub mod reqwest_impl;
pub mod transport;

pub use body::BodyStream;
pub use collect::{MAX_RESPONSE_BODY_BYTES, collect_body};
pub use error::TransportError;
pub use http_transport::HttpTransport;
pub use reqwest_impl::ReqwestTransport;
pub use transport::Transport;
