//! Smoke test: `MockHttpTransport` receives a request and returns a canned
//! response, giving downstream test suites a confidence baseline for the
//! mock surface.

#![cfg(feature = "__test-mocks")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use http::{Request, Response, StatusCode};

use clauders::error::TransportError;
use clauders::transport::{BodyStream, HttpTransport, MockHttpTransport};

fn canned_body(payload: &'static [u8]) -> BodyStream {
    struct Once(Option<Bytes>);

    impl Stream for Once {
        type Item = Result<Bytes, TransportError>;
        fn poll_next(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.0.take().map(Ok))
        }
    }

    Box::pin(Once(Some(Bytes::from_static(payload))))
}

#[tokio::test]
async fn mock_returns_canned_response() {
    let mut transport = MockHttpTransport::new();
    transport
        .expect_send()
        .times(1)
        .returning(|_req| {
            let mut response = Response::new(canned_body(b"hello"));
            *response.status_mut() = StatusCode::OK;
            Ok(response)
        });

    let req = Request::builder()
        .method("GET")
        .uri("https://example/")
        .body(Bytes::new())
        .unwrap();

    let resp = transport.send(req).await.expect("mock returned canned response");
    assert_eq!(resp.status(), StatusCode::OK);
}
