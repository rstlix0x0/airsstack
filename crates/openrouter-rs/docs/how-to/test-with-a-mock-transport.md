# How to test with a mock transport

The transport is a generic parameter on `Client<T>`, so you can substitute a
fake and exercise the entire SDK — URL building, header construction, status
routing, decoding, cache-header parsing, SSE framing — without a network.

> **The crate's own `MockHttpTransport` is not available to you.** It lives in a
> `#[cfg(test)]`-only private module and depends on `mockall`, a dev-dependency.
> It exists for this crate's unit tests. Downstream, write the ten-line fake
> below instead.

## Write a fake transport

Implement `airs_transport::Transport` with the HTTP associated types. The
blanket impl in `http_transport` then grants `HttpTransport` automatically, so
`Client::builder_with_transport` accepts it.

```rust
use bytes::Bytes;
use futures_core::Stream;
use http::{Request, Response, StatusCode};
use std::pin::Pin;
use std::task::{Context, Poll};

use openrouter_rs::transport::{BodyStream, Transport, TransportError};

/// Replays one canned response, whatever the request.
struct CannedTransport {
    status: StatusCode,
    body: &'static [u8],
}

/// A BodyStream that yields its payload once, then ends.
fn body_from(payload: &'static [u8]) -> BodyStream {
    struct Once(Option<Bytes>);
    impl Stream for Once {
        type Item = Result<Bytes, TransportError>;
        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.0.take().map(Ok))
        }
    }
    Box::pin(Once(Some(Bytes::from_static(payload))))
}

#[async_trait::async_trait]
impl Transport for CannedTransport {
    type Request = Request<Bytes>;
    type Response = Response<BodyStream>;
    type Error = TransportError;

    async fn send(&self, _req: Request<Bytes>) -> Result<Response<BodyStream>, TransportError> {
        let mut resp = Response::new(body_from(self.body));
        *resp.status_mut() = self.status;
        Ok(resp)
    }
}
```

You need `async-trait`, `bytes`, `futures-core`, and `http` as dev-dependencies —
the same versions the crate uses.

## Drive a happy path

```rust
const HAPPY: &[u8] = br#"{"id":"gen-1","object":"chat.completion","created":1,
    "model":"openai/gpt-4o","choices":[{"index":0,
    "message":{"role":"assistant","content":"4"},"finish_reason":"stop"}],
    "usage":{"prompt_tokens":5,"completion_tokens":1,"total_tokens":6}}"#;

#[tokio::test]
async fn decodes_a_completion() {
    let transport = CannedTransport { status: StatusCode::OK, body: HAPPY };
    let client = Client::builder_with_transport(transport)
        .api_key(ApiKey::new("sk-or-v1-test").unwrap())
        .build()
        .unwrap();

    let req = ChatRequest::builder()
        .model(ModelId::custom("openai/gpt-4o").unwrap())
        .messages(vec![Message::user("2+2?")])
        .build();

    let completion = client.chat().send(req).await.unwrap();
    assert_eq!(completion.choices[0].message.content.as_deref(), Some("4"));
    assert_eq!(completion.usage.unwrap().total_tokens, 6);
}
```

Note `Client::builder_with_transport` is infallible — no `?` on that call.

## Assert on the outgoing request

Because `send` receives the fully-built `Request<Bytes>`, your fake can inspect
headers, method, URI, and body before answering:

```rust
async fn send(&self, req: Request<Bytes>) -> Result<Response<BodyStream>, TransportError> {
    assert_eq!(req.method(), http::Method::POST);
    assert!(req.uri().to_string().ends_with("/chat/completions"));

    let auth = req.headers().get("authorization").unwrap().to_str().unwrap();
    assert!(auth.starts_with("Bearer "));

    let sent: serde_json::Value = serde_json::from_slice(req.body()).unwrap();
    assert_eq!(sent["model"], "openai/gpt-4o");
    assert!(sent.get("stream").is_none(), "non-streaming send must omit the flag");

    // ... return a canned response
}
```

## Exercise the error paths

Return a non-2xx status with an error envelope and check that it routes to the
variant you expect:

```rust
let transport = CannedTransport {
    status: StatusCode::TOO_MANY_REQUESTS,
    body: br#"{"error":{"code":429,"message":"slow down"}}"#,
};
// ...
let err = client.chat().send(req).await.unwrap_err();
assert!(matches!(err, Error::RateLimit { .. }));
```

The full routing table is in
[reference/errors.md](../reference/errors.md#how-a-non-2xx-response-is-classified).

## Exercise streaming

Feed raw SSE text as the body. The framing is parsed for real, so this tests the
`ChatStream` behaviour end to end, including termination:

```rust
const SSE: &[u8] = concat!(
    "data: {\"id\":\"g\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"x/y\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hel\"}}]}\n\n",
    "data: {\"id\":\"g\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"x/y\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"}}]}\n\n",
    "data: [DONE]\n\n",
).as_bytes();
```

Collect the deltas and assert you got `"Hello"`, then assert the next poll is
`None`.

## Exercise the edge cache

Set the response headers your fake returns and check they land on the envelope:

```rust
resp.headers_mut().insert("x-openrouter-cache-status", "HIT".parse().unwrap());
resp.headers_mut().insert("x-openrouter-cache-age", "12".parse().unwrap());
```

```rust
let out = client.chat().send_cached(req, ResponseCache::enabled()).await.unwrap();
assert_eq!(out.status, CacheStatus::Hit);
assert_eq!(out.age_secs, Some(12));
```

## Keeping your code testable

Write functions generic over the transport rather than pinning `DefaultClient`:

```rust
use openrouter_rs::transport::HttpTransport;

async fn summarise<T: HttpTransport>(client: &Client<T>, text: &str) -> Result<String, Error> {
    // ...
}
```

Production passes `Client<ReqwestTransport>`; tests pass `Client<CannedTransport>`.
Both monomorphise — there is no dynamic dispatch and no runtime cost for the
seam.

## For contributors to this crate

Inside the crate, `crate::test_support::MockHttpTransport` is a `mockall` double
with the full expectation API:

```rust
let mut transport = MockHttpTransport::new();
transport.expect_send().times(1).returning(|_req| { /* ... */ });
```

It is `#[cfg(test)]`-only and gated behind no Cargo feature — the crate declares
none.
