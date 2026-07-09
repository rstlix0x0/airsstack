//! Test-only doubles for this crate's unit tests.
//!
//! `MockHttpTransport` is a `mockall`-generated fake of the transport
//! [`airs_transport::Transport`] contract fixed to the HTTP associated types.
//! Compiled only under `cfg(test)`; `mockall` is a dev-dependency, so neither
//! this module nor `mockall` is present in a normal build. The un-gated blanket
//! impl in `airs_transport::http_transport` makes the generated mock an
//! `HttpTransport` automatically.

use bytes::Bytes;
use http::{Request, Response};

use airs_transport::{BodyStream, Transport, TransportError};

mockall::mock! {
    /// Mock implementation of [`airs_transport::Transport`] (HTTP types) for tests.
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
