//! `MockHttpTransport` — `mockall`-generated fake of [`HttpTransport`].
//!
//! No inline tests per the unit-test-mandate exemption #4 (the body is a
//! code-generation macro). Gated behind the private `__test-mocks` feature;
//! production builds never compile this module.
//!
//! Responsibilities:
//! - Emit `MockHttpTransport` via [`mockall::mock!`] so test code sets
//!   expectations on `expect_send()` without a hand-rolled fake.

use bytes::Bytes;
use http::{Request, Response};

use super::{BodyStream, HttpTransport};
use crate::error::TransportError;

mockall::mock! {
    /// Mock implementation of [`HttpTransport`] for tests.
    ///
    /// Set expectations with `expect_send()`; see the `mockall` docs for the
    /// full expectation API.
    pub HttpTransport {}

    #[async_trait::async_trait]
    impl HttpTransport for HttpTransport {
        async fn send(
            &self,
            req: Request<Bytes>,
        ) -> Result<Response<BodyStream>, TransportError>;
    }
}
