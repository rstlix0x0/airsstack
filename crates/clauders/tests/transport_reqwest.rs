//! Black-box integration test: `ReqwestTransport` against a local
//! `wiremock` server.
//!
//! Covers the happy path (200 with a body the test consumes via the
//! `BodyStream` adapter) and the boundary case where a 4xx response is
//! NOT a transport error — it surfaces as `Ok` and the layer above
//! decides what to do with the status.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bytes::Bytes;
use futures_util::StreamExt;
use http::{Request, StatusCode};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clauders::error::TransportError;
use clauders::transport::{BodyStream, HttpTransport, ReqwestTransport};

async fn collect_body(mut s: BodyStream) -> Result<Vec<u8>, TransportError> {
    let mut out = Vec::new();
    while let Some(chunk) = s.next().await {
        out.extend_from_slice(&chunk?);
    }
    Ok(out)
}

#[tokio::test]
async fn round_trips_200_with_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/echo"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-custom", "hello")
                .set_body_string("pong"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let transport = ReqwestTransport::default();
    let url = format!("{}/echo", server.uri());

    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header("content-type", "application/json")
        .body(Bytes::from_static(b"ping"))
        .unwrap();

    let resp = transport.send(req).await.expect("transport ok");
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("x-custom").unwrap(), "hello");

    let (_, body) = resp.into_parts();
    let bytes = collect_body(body).await.unwrap();
    assert_eq!(bytes, b"pong");
}

#[tokio::test]
async fn surfaces_404_as_ok_response() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let transport = ReqwestTransport::default();
    let url = format!("{}/missing", server.uri());
    let req = Request::builder()
        .method("GET")
        .uri(url)
        .body(Bytes::new())
        .unwrap();

    let resp = transport
        .send(req)
        .await
        .expect("transport ok — 4xx is not a transport error");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
