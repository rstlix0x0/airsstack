//! `ChatResource` — the entry point for `POST /chat/completions` requests.
//!
//! Exists as its own file so request dispatch is separate from the wire-format
//! type definitions (`request.rs` / `response.rs`) and the module table of
//! contents. The handle borrows a `Client<T>` and is created at the call site.
//!
//! Responsibilities:
//! - Define [`ChatResource`], the short-lived handle vended by
//!   [`crate::client::Client::chat`].
//! - Implement [`ChatResource::send`] — serialize the request, build the URL
//!   and headers, dispatch through the transport, and decode the response.
//!
//! Not responsible for:
//! - Retry / backoff — deferred to a later layer.
//! - Streaming — the streaming entry point lands with that feature.
//! - Body collection and error decoding — those live in `crate::wire_helpers`.

use bytes::Bytes;
use http::{Method, Request};

use crate::chat::request::ChatRequest;
use crate::chat::response::ChatCompletion;
use crate::client::Client;
use crate::error::Error;
use crate::headers as h;
use crate::transport::{BodyStream, HttpTransport};
use crate::wire_helpers::{MAX_RESPONSE_BODY_BYTES, collect_body, decode_api_error_from_parts};

#[cfg(feature = "streaming")]
use crate::chat::stream::ChatStream;

/// Path joined onto the configured base URL for chat-completion calls.
///
/// No leading slash — relies on the additive-join semantics documented on
/// `BaseUrl::join` (a base whose path ends with `/`).
const CHAT_PATH: &str = "chat/completions";

/// Short-lived handle for the chat-completions endpoint, borrowing a `Client<T>`.
///
/// Obtain via [`Client::chat`]; do not construct directly.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "transport-reqwest")]
/// # async fn example() -> Result<(), openrouter_rs::error::Error> {
/// use openrouter_rs::Client;
/// use openrouter_rs::chat::{ChatRequest, Message};
/// use openrouter_rs::types::{ApiKey, ModelId};
///
/// let client = Client::builder()?
///     .api_key(ApiKey::new("sk-or-v1-...").unwrap())
///     .build()?;
/// let req = ChatRequest::builder()
///     .model(ModelId::custom("openai/gpt-4o").unwrap())
///     .messages(vec![Message::user("Hello!")])
///     .build();
/// let completion = client.chat().send(req).await?;
/// println!("{}", completion.choices.len());
/// # Ok(())
/// # }
/// ```
pub struct ChatResource<'a, T: HttpTransport> {
    pub(crate) client: &'a Client<T>,
}

impl<T: HttpTransport> ChatResource<'_, T> {
    /// Send a `ChatRequest` and decode the non-streaming response.
    ///
    /// # Errors
    /// - [`Error::Serde`] — request serialization fails, or a 2xx body cannot
    ///   be decoded as a [`ChatCompletion`].
    /// - [`Error::Transport`] — a network-level failure occurs while sending
    ///   or reading the response body.
    /// - [`Error::RateLimit`] — the API returns HTTP 429.
    /// - [`Error::Moderation`] — the API returns an HTTP 403 moderation block.
    /// - [`Error::Provider`] — an upstream provider error is passed through.
    /// - [`Error::Api`] — any other non-2xx status with a decodable envelope.
    /// - [`Error::UndecodableApiError`] — a non-2xx status whose body is not a
    ///   recognized error envelope.
    /// - [`Error::InvalidRequest`] — the base URL cannot be joined with the
    ///   chat path, or the HTTP request cannot be constructed.
    pub async fn send(&self, req: ChatRequest) -> Result<ChatCompletion, Error> {
        let raw = self.send_request(req).await?;
        self.decode_response(raw).await
    }

    async fn send_request(&self, req: ChatRequest) -> Result<http::Response<BodyStream>, Error> {
        let body = serde_json::to_vec(&req).map_err(|e| Error::Serde {
            context: "ChatRequest",
            source: e,
        })?;

        let url = self
            .client
            .config()
            .base_url()
            .join(CHAT_PATH)
            .map_err(|e| Error::InvalidRequest(format!("failed to build chat URL: {e}")))?;

        let mut builder = Request::builder()
            .method(Method::POST)
            .uri(url.as_str())
            .header(h::CONTENT_TYPE, h::APPLICATION_JSON)
            .header(h::ACCEPT, h::APPLICATION_JSON);

        if let Some(key) = self.client.auth().api_key() {
            builder = builder.header(
                h::AUTHORIZATION,
                format!("{}{}", h::BEARER_PREFIX, key.expose_secret()),
            );
        }
        if let Some(referer) = self.client.config().http_referer() {
            builder = builder.header(h::HTTP_REFERER, referer);
        }
        if let Some(title) = self.client.config().app_title() {
            builder = builder.header(h::X_TITLE, title);
        }

        let http_req = builder
            .body(Bytes::from(body))
            .map_err(|e| Error::InvalidRequest(format!("failed to build HTTP request: {e}")))?;

        self.client
            .inner
            .transport
            .send(http_req)
            .await
            .map_err(Error::Transport)
    }

    async fn decode_response(
        &self,
        resp: http::Response<BodyStream>,
    ) -> Result<ChatCompletion, Error> {
        let (parts, body) = resp.into_parts();
        let body_bytes = collect_body(body, MAX_RESPONSE_BODY_BYTES)
            .await
            .map_err(Error::Transport)?;

        if parts.status.is_success() {
            serde_json::from_slice::<ChatCompletion>(&body_bytes).map_err(|e| Error::Serde {
                context: "ChatCompletion",
                source: e,
            })
        } else {
            Err(decode_api_error_from_parts(
                parts.status,
                &parts.headers,
                &body_bytes,
            ))
        }
    }

    /// Send a `ChatRequest` as a streaming request and return a [`ChatStream`].
    ///
    /// The response status is checked eagerly: a non-2xx response is decoded as
    /// an error immediately, and only a 2xx response yields the stream handle.
    /// The returned stream yields one [`crate::chat::StreamChunk`] per SSE
    /// `data:` line, terminates on `data: [DONE]`, and is terminal once it
    /// yields an error.
    ///
    /// # Errors
    /// - [`Error::Serde`] — request serialization fails.
    /// - [`Error::Transport`] — a network-level failure occurs before headers
    ///   arrive, or while reading a non-2xx error body.
    /// - [`Error::RateLimit`] — the API returns HTTP 429.
    /// - [`Error::Moderation`] — the API returns an HTTP 403 moderation block.
    /// - [`Error::Provider`] — an upstream provider error is passed through.
    /// - [`Error::Api`] — any other non-2xx status with a decodable envelope.
    /// - [`Error::UndecodableApiError`] — a non-2xx status whose body is not a
    ///   recognized error envelope.
    /// - [`Error::InvalidRequest`] — the base URL cannot be joined with the
    ///   chat path, or the HTTP request cannot be constructed.
    #[cfg(feature = "streaming")]
    #[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
    pub async fn stream(&self, mut req: ChatRequest) -> Result<ChatStream, Error> {
        req.stream = true;

        let body = serde_json::to_vec(&req).map_err(|e| Error::Serde {
            context: "ChatRequest",
            source: e,
        })?;

        let url = self
            .client
            .config()
            .base_url()
            .join(CHAT_PATH)
            .map_err(|e| Error::InvalidRequest(format!("failed to build chat URL: {e}")))?;

        let mut builder = Request::builder()
            .method(Method::POST)
            .uri(url.as_str())
            .header(h::CONTENT_TYPE, h::APPLICATION_JSON)
            .header(h::ACCEPT, h::TEXT_EVENT_STREAM);

        if let Some(key) = self.client.auth().api_key() {
            builder = builder.header(
                h::AUTHORIZATION,
                format!("{}{}", h::BEARER_PREFIX, key.expose_secret()),
            );
        }
        if let Some(referer) = self.client.config().http_referer() {
            builder = builder.header(h::HTTP_REFERER, referer);
        }
        if let Some(title) = self.client.config().app_title() {
            builder = builder.header(h::X_TITLE, title);
        }

        let http_req = builder
            .body(Bytes::from(body))
            .map_err(|e| Error::InvalidRequest(format!("failed to build HTTP request: {e}")))?;

        let resp = self
            .client
            .inner
            .transport
            .send(http_req)
            .await
            .map_err(Error::Transport)?;

        let (parts, body) = resp.into_parts();

        if !parts.status.is_success() {
            let body_bytes = collect_body(body, MAX_RESPONSE_BODY_BYTES)
                .await
                .map_err(Error::Transport)?;
            return Err(decode_api_error_from_parts(
                parts.status,
                &parts.headers,
                &body_bytes,
            ));
        }

        Ok(ChatStream::new(body))
    }
}

#[cfg(all(test, feature = "streaming", feature = "__test-mocks"))]
mod stream_tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::panic,
        reason = "test-only panics on wrong-variant matches; a panic is the intended failure signal"
    )]

    use std::pin::Pin;
    use std::task::{Context, Poll};

    use bytes::Bytes;
    use futures_core::Stream;
    use futures_util::StreamExt;
    use http::{Response, StatusCode};

    use crate::chat::{ChatRequest, Message};
    use crate::client::Client;
    use crate::error::{Error, TransportError};
    use crate::transport::{BodyStream, MockHttpTransport};
    use crate::types::{ApiKey, ModelId};

    fn body_from(payload: Vec<u8>) -> BodyStream {
        struct Once(Option<Bytes>);
        impl Stream for Once {
            type Item = Result<Bytes, TransportError>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                Poll::Ready(self.0.take().map(Ok))
            }
        }
        Box::pin(Once(Some(Bytes::from(payload))))
    }

    fn client_with(transport: MockHttpTransport) -> Client<MockHttpTransport> {
        Client::builder_with_transport(transport)
            .api_key(ApiKey::new("sk-or-v1-test").unwrap())
            .build()
            .unwrap()
    }

    fn minimal_request() -> ChatRequest {
        ChatRequest::builder()
            .model(ModelId::custom("openai/gpt-4o").unwrap())
            .messages(vec![Message::user("hi")])
            .build()
    }

    const SSE: &str = concat!(
        "data: {\"id\":\"g\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"openai/gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"g\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"openai/gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\n",
        "data: [DONE]\n\n",
    );

    #[tokio::test]
    async fn stream_2xx_yields_chunks() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from(SSE.as_bytes().to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let mut stream = client.chat().stream(minimal_request()).await.unwrap();

        let mut content = String::new();
        let mut total = None;
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            if let Some(c) = &chunk.choices[0].delta.content {
                content.push_str(c);
            }
            if let Some(u) = chunk.usage {
                total = Some(u.total_tokens);
            }
        }
        assert_eq!(content, "Hello");
        assert_eq!(total, Some(3));
    }

    #[tokio::test]
    async fn stream_non_2xx_returns_error_before_streaming() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let body = br#"{"error":{"code":400,"message":"bad request"}}"#.to_vec();
            let mut resp = Response::new(body_from(body));
            *resp.status_mut() = StatusCode::BAD_REQUEST;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client.chat().stream(minimal_request()).await.unwrap_err();
        match err {
            Error::Api { status, .. } => assert_eq!(status, 400),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }
}

#[cfg(all(test, feature = "__test-mocks"))]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::panic,
        reason = "test-only panics on wrong-variant matches; a panic is the intended failure signal"
    )]

    use std::pin::Pin;
    use std::task::{Context, Poll};

    use bytes::Bytes;
    use futures_core::Stream;
    use http::{Response, StatusCode};

    use crate::chat::{ChatRequest, Message};
    use crate::client::Client;
    use crate::error::{Error, TransportError};
    use crate::transport::{BodyStream, MockHttpTransport};
    use crate::types::{ApiKey, ModelId};

    fn body_from(payload: Vec<u8>) -> BodyStream {
        struct Once(Option<Bytes>);
        impl Stream for Once {
            type Item = Result<Bytes, TransportError>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                Poll::Ready(self.0.take().map(Ok))
            }
        }
        Box::pin(Once(Some(Bytes::from(payload))))
    }

    fn client_with(transport: MockHttpTransport) -> Client<MockHttpTransport> {
        Client::builder_with_transport(transport)
            .api_key(ApiKey::new("sk-or-v1-test").unwrap())
            .build()
            .unwrap()
    }

    fn minimal_request() -> ChatRequest {
        ChatRequest::builder()
            .model(ModelId::custom("openai/gpt-4o").unwrap())
            .messages(vec![Message::user("hi")])
            .build()
    }

    const HAPPY: &[u8] = br#"{"id":"gen-1","object":"chat.completion","created":1,
        "model":"openai/gpt-4o","choices":[{"index":0,
        "message":{"role":"assistant","content":"4"},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":5,"completion_tokens":1,"total_tokens":6}}"#;

    #[tokio::test]
    async fn send_2xx_decodes_completion() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from(HAPPY.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let completion = client.chat().send(minimal_request()).await.unwrap();

        assert_eq!(completion.id, "gen-1");
        assert_eq!(completion.choices.len(), 1);
        assert_eq!(completion.choices[0].message.content.as_deref(), Some("4"));
        assert_eq!(completion.usage.unwrap().total_tokens, 6);
    }

    #[tokio::test]
    async fn send_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let body = br#"{"error":{"code":400,"message":"bad request"}}"#.to_vec();
            let mut resp = Response::new(body_from(body));
            *resp.status_mut() = StatusCode::BAD_REQUEST;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client.chat().send(minimal_request()).await.unwrap_err();

        match err {
            Error::Api { status, .. } => assert_eq!(status, 400),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_429_returns_rate_limit() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let body = br#"{"error":{"code":429,"message":"slow"}}"#.to_vec();
            let mut resp = Response::new(body_from(body));
            *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client.chat().send(minimal_request()).await.unwrap_err();
        assert!(matches!(err, Error::RateLimit { .. }));
    }
}
