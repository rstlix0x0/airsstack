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
        async fn send(&self, _req: Request<Bytes>) -> Result<Response<BodyStream>, TransportError> {
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
