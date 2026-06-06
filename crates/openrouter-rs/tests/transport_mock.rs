//! Exercises `MockHttpTransport` through the `HttpTransport` seam.
#![cfg(feature = "__test-mocks")]
#![expect(
    clippy::expect_used,
    reason = "integration test asserts on known-valid fixtures"
)]

use bytes::Bytes;
use futures_util::StreamExt;
use http::{Request, Response, StatusCode};
use openrouter_rs::transport::{BodyStream, MockHttpTransport, Transport};

fn canned_response(body: &'static [u8]) -> Response<BodyStream> {
    let stream: BodyStream = Box::pin(futures_util::stream::once(async move {
        Ok(Bytes::from_static(body))
    }));
    let mut resp = Response::new(stream);
    *resp.status_mut() = StatusCode::OK;
    resp
}

#[tokio::test]
async fn mock_send_returns_canned_body() {
    let mut transport = MockHttpTransport::new();
    transport
        .expect_send()
        .returning(|_req| Ok(canned_response(b"{\"ok\":true}")));

    let req = Request::builder()
        .method("POST")
        .uri("https://openrouter.ai/api/v1/chat/completions")
        .body(Bytes::new())
        .expect("request builds");

    let resp = transport.send(req).await.expect("mock send ok");
    assert_eq!(resp.status(), StatusCode::OK);

    let mut body = resp.into_body();
    let mut collected = Vec::new();
    while let Some(chunk) = body.next().await {
        collected.extend_from_slice(&chunk.expect("body chunk"));
    }
    assert_eq!(collected, b"{\"ok\":true}");
}
