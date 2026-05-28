//! `MockHttpTransport` — `mockall`-generated fake of [`HttpTransport`] for tests.
//!
//! Exists as its own module so the `mockall::mock!` macro invocation
//! (which expands to a non-trivial struct + impl block) does not mix with
//! the export table in `mod.rs`. Gated behind the private `__test-mocks`
//! feature (double-underscore prefix marks it caller-private); production
//! builds never compile this module.
//!
//! Responsibilities:
//! - Emit the `MockHttpTransport` type via [`mockall::mock!`] so downstream
//!   test code can set expectations on `expect_send()` without writing a
//!   hand-rolled fake.
//!
//! Not responsible for:
//! - Providing test scaffolding (canned responses, body builders) — tests
//!   construct those themselves to keep the mock surface minimal.

use bytes::Bytes;
use http::{Request, Response};

use super::{BodyStream, HttpTransport};
use crate::error::TransportError;

mockall::mock! {
    /// Mock implementation of [`HttpTransport`] for use in tests.
    ///
    /// Set expectations with `expect_send()`; see the `mockall` crate
    /// docs for the full expectation API.
    pub HttpTransport {}

    #[async_trait::async_trait]
    impl HttpTransport for HttpTransport {
        async fn send(
            &self,
            req: Request<Bytes>,
        ) -> Result<Response<BodyStream>, TransportError>;
    }
}
