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
