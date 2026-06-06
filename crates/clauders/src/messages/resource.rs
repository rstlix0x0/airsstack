//! `MessagesResource` — the entry point for `POST /v1/messages` requests.
//!
//! Exists as its own module so request dispatch logic is separate from
//! the mod.rs table of contents and the wire-format type definitions in
//! `request.rs` / `response.rs`.
//!
//! Responsibilities:
//! - Define [`MessagesResource`], the short-lived handle vended by
//!   [`crate::client::Client::messages`].
//! - Implement `create` — serialize the request, assemble headers, dispatch
//!   through the transport, decode the response.
//! - Implement `stream` (behind `messages-streaming`) — serialize, send with
//!   `Accept: text/event-stream`, check for non-2xx before yielding the
//!   stream, and wrap the body in a [`crate::messages::MessageStream`].
//!
//! Not responsible for:
//! - Retry logic — the client layer owns that.
//! - Auth schemes beyond API-key — `Auth` handles that.
//! - Body collection and error decoding helpers — those live in a
//!   shared internal module used across all resource modules.

use bytes::Bytes;
use http::{Method, Request};

use crate::client::Client;
use crate::error::Error;
use crate::headers as h;
use crate::transport::{BodyStream, HttpTransport, MAX_RESPONSE_BODY_BYTES, collect_body};
use crate::wire_helpers::decode_api_error_from_parts;

use super::request::MessageRequest;
use super::response::Message;

/// Path appended to the configured base URL for Messages API calls.
///
/// Value: `v1/messages` (no leading slash — relies on `BaseUrl::join`
/// segment-resolution semantics documented on that method).
const MESSAGES_PATH: &str = "v1/messages";

/// Short-lived handle for the Messages API, borrowing a `Client<T>`.
///
/// Obtain via [`Client::messages`]; do not construct directly.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "transport-reqwest")]
/// # async fn example() -> Result<(), clauders::error::Error> {
/// # use clauders::Client;
/// # use clauders::messages::MessageRequest;
/// # use clauders::types::{ApiKey, MaxTokens, ModelId};
/// let client = Client::builder()?
///     .api_key(ApiKey::new("sk-ant-…").unwrap())
///     .build()?;
/// let req = MessageRequest::builder()
///     .model(ModelId::claude_sonnet_4_5())
///     .max_tokens(MaxTokens::new(1024).unwrap())
///     .add_user_text("Hello!")
///     .build();
/// let msg = client.messages().create(req).await?;
/// println!("{}", msg.content.len());
/// # Ok(())
/// # }
/// ```
pub struct MessagesResource<'a, T: HttpTransport> {
    pub(crate) client: &'a Client<T>,
}

impl<T: HttpTransport> MessagesResource<'_, T> {
    /// Send a `MessageRequest` and decode the response.
    ///
    /// # Errors
    /// - [`Error::Serde`] — request body serialization fails, or a 2xx
    ///   response body cannot be decoded as a [`Message`].
    /// - [`Error::Transport`] — a network-level failure occurs while sending
    ///   the request or reading the response body.
    /// - [`Error::Api`] — the API returns a non-2xx status with a decodable
    ///   error envelope.
    /// - [`Error::UndecodableApiError`] — the API returns a non-2xx status
    ///   whose body cannot be parsed as a known error envelope.
    /// - [`Error::InvalidRequest`] — the configured base URL cannot be joined
    ///   with the messages path, or the HTTP request cannot be constructed.
    pub async fn create(&self, req: MessageRequest) -> Result<Message, Error> {
        let raw = self.send_request(req).await?;
        self.decode_response(raw).await
    }

    async fn send_request(&self, req: MessageRequest) -> Result<http::Response<BodyStream>, Error> {
        let body = serde_json::to_vec(&req).map_err(|e| Error::Serde {
            context: "MessageRequest",
            source: e,
        })?;

        let url = self
            .client
            .config()
            .base_url()
            .join(MESSAGES_PATH)
            .map_err(|e| Error::InvalidRequest(format!("failed to build messages URL: {e}")))?;

        let mut builder = Request::builder()
            .method(Method::POST)
            .uri(url.as_str())
            .header(h::CONTENT_TYPE, h::APPLICATION_JSON)
            .header(h::ACCEPT, h::APPLICATION_JSON)
            .header(
                h::ANTHROPIC_VERSION,
                self.client.config().anthropic_version().as_str(),
            );

        if let Some(key) = self.client.auth().api_key() {
            builder = builder.header(h::X_API_KEY, key.expose_secret());
        }

        let beta = self.client.config().anthropic_beta();
        if !beta.is_empty() {
            let joined = beta
                .iter()
                .map(crate::types::BetaHeader::as_str)
                .collect::<Vec<_>>()
                .join(",");
            builder = builder.header(h::ANTHROPIC_BETA, joined);
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

    async fn decode_response(&self, resp: http::Response<BodyStream>) -> Result<Message, Error> {
        let (parts, body) = resp.into_parts();

        let body_bytes = collect_body(body, MAX_RESPONSE_BODY_BYTES)
            .await
            .map_err(Error::Transport)?;

        if parts.status.is_success() {
            serde_json::from_slice::<Message>(&body_bytes).map_err(|e| Error::Serde {
                context: "Message",
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

    /// Send a streaming `MessageRequest` and return a [`crate::messages::MessageStream`].
    ///
    /// The HTTP response status is checked eagerly before the stream is
    /// returned. A non-2xx response is decoded as an error immediately;
    /// only a 2xx response yields the stream handle.
    ///
    /// # Errors
    ///
    /// - [`Error::Serde`] — request body serialization fails.
    /// - [`Error::Transport`] — a network-level failure occurs before headers arrive.
    /// - [`Error::Api`] — the API returns a non-2xx status with a decodable error envelope.
    /// - [`Error::UndecodableApiError`] — the API returns a non-2xx status whose
    ///   body cannot be parsed as a known error envelope.
    /// - [`Error::InvalidRequest`] — the configured base URL cannot be joined with
    ///   the messages path, or the HTTP request cannot be constructed.
    #[cfg(feature = "messages-streaming")]
    #[cfg_attr(docsrs, doc(cfg(feature = "messages-streaming")))]
    pub async fn stream(
        &self,
        mut req: MessageRequest,
    ) -> Result<super::streaming::MessageStream, Error> {
        req.stream = true;

        let body = serde_json::to_vec(&req).map_err(|e| Error::Serde {
            context: "MessageRequest",
            source: e,
        })?;

        let url = self
            .client
            .config()
            .base_url()
            .join(MESSAGES_PATH)
            .map_err(|e| Error::InvalidRequest(format!("failed to build messages URL: {e}")))?;

        let mut builder = http::Request::builder()
            .method(http::Method::POST)
            .uri(url.as_str())
            .header(h::CONTENT_TYPE, h::APPLICATION_JSON)
            .header(h::ACCEPT, h::TEXT_EVENT_STREAM)
            .header(
                h::ANTHROPIC_VERSION,
                self.client.config().anthropic_version().as_str(),
            );

        if let Some(key) = self.client.auth().api_key() {
            builder = builder.header(h::X_API_KEY, key.expose_secret());
        }

        let beta = self.client.config().anthropic_beta();
        if !beta.is_empty() {
            let joined = beta
                .iter()
                .map(crate::types::BetaHeader::as_str)
                .collect::<Vec<_>>()
                .join(",");
            builder = builder.header(h::ANTHROPIC_BETA, joined);
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

        let (parts, body_stream) = resp.into_parts();

        if !parts.status.is_success() {
            let body_bytes = collect_body(body_stream, MAX_RESPONSE_BODY_BYTES)
                .await
                .map_err(Error::Transport)?;

            return Err(decode_api_error_from_parts(
                parts.status,
                &parts.headers,
                &body_bytes,
            ));
        }

        Ok(super::streaming::MessageStream::new(body_stream))
    }

    /// Return a handle for the Message Batches API.
    ///
    /// The returned [`super::batches::resource::BatchesResource`] borrows
    /// from `self`; create it close to use and drop it after.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "transport-reqwest")]
    /// # async fn example() -> Result<(), clauders::error::Error> {
    /// # use clauders::Client;
    /// # use clauders::messages::{BatchRequest, MessageRequest};
    /// # use clauders::types::{ApiKey, CustomRequestId, MaxTokens, ModelId};
    /// let client = Client::builder()?
    ///     .api_key(ApiKey::new("sk-ant-…").unwrap())
    ///     .build()?;
    /// let batches = client.messages().batches();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "messages-batches")]
    #[cfg_attr(docsrs, doc(cfg(feature = "messages-batches")))]
    #[must_use]
    pub const fn batches(&self) -> super::batches::resource::BatchesResource<'_, T> {
        super::batches::resource::BatchesResource {
            client: self.client,
        }
    }

    /// Count the tokens a request would consume without generating a response.
    ///
    /// Sends `POST /v1/messages/count_tokens` with the subset of fields the
    /// endpoint accepts. The `max_tokens`, `temperature`, `top_p`, `top_k`,
    /// `stop_sequences`, `metadata`, and `stream` fields on the supplied
    /// [`MessageRequest`] are intentionally omitted because the endpoint
    /// rejects unrecognised fields.
    ///
    /// # Errors
    /// - [`Error::Serde`] — body serialization fails or the 2xx response
    ///   cannot be decoded as a [`super::token_counting::TokenCount`].
    /// - [`Error::Transport`] — a network-level failure occurs.
    /// - [`Error::Api`] — the API returns a non-2xx status with a decodable
    ///   error envelope.
    /// - [`Error::UndecodableApiError`] — the API returns a non-2xx status
    ///   whose body cannot be parsed.
    /// - [`Error::InvalidRequest`] — the base URL cannot be joined with the
    ///   count-tokens path, or the HTTP request cannot be constructed.
    #[cfg(feature = "messages-token-counting")]
    #[cfg_attr(docsrs, doc(cfg(feature = "messages-token-counting")))]
    pub async fn count_tokens(
        &self,
        req: MessageRequest,
    ) -> Result<super::token_counting::TokenCount, Error> {
        use super::token_counting::CountTokensBody;

        let body_struct = CountTokensBody::from_request(&req);
        let body_bytes = serde_json::to_vec(&body_struct).map_err(|e| Error::Serde {
            context: "CountTokensBody",
            source: e,
        })?;

        let url = self
            .client
            .config()
            .base_url()
            .join("v1/messages/count_tokens")
            .map_err(|e| Error::InvalidRequest(format!("failed to build count_tokens URL: {e}")))?;

        let mut builder = http::Request::builder()
            .method(http::Method::POST)
            .uri(url.as_str())
            .header(h::CONTENT_TYPE, h::APPLICATION_JSON)
            .header(h::ACCEPT, h::APPLICATION_JSON)
            .header(
                h::ANTHROPIC_VERSION,
                self.client.config().anthropic_version().as_str(),
            );

        if let Some(key) = self.client.auth().api_key() {
            builder = builder.header(h::X_API_KEY, key.expose_secret());
        }

        let beta = self.client.config().anthropic_beta();
        if !beta.is_empty() {
            let joined = beta
                .iter()
                .map(crate::types::BetaHeader::as_str)
                .collect::<Vec<_>>()
                .join(",");
            builder = builder.header(h::ANTHROPIC_BETA, joined);
        }

        let http_req = builder
            .body(Bytes::from(body_bytes))
            .map_err(|e| Error::InvalidRequest(format!("failed to build HTTP request: {e}")))?;

        let resp = self
            .client
            .inner
            .transport
            .send(http_req)
            .await
            .map_err(Error::Transport)?;

        let (parts, body_stream) = resp.into_parts();
        let bytes = collect_body(body_stream, MAX_RESPONSE_BODY_BYTES)
            .await
            .map_err(Error::Transport)?;

        if parts.status.is_success() {
            serde_json::from_slice::<super::token_counting::TokenCount>(&bytes).map_err(|e| {
                Error::Serde {
                    context: "TokenCount",
                    source: e,
                }
            })
        } else {
            Err(decode_api_error_from_parts(
                parts.status,
                &parts.headers,
                &bytes,
            ))
        }
    }
}

#[cfg(all(test, feature = "__test-mocks"))]
mod tests {
    //! Tests for the decode branches in `MessagesResource`: 2xx success
    //! path and non-2xx error path. The body-collection and error-decoding
    //! helpers are tested in their own module (`crate::wire_helpers::tests`);
    //! these tests focus on the status-code branch and the
    //! count-tokens path exercised through a mock transport.

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

    use crate::error::{Error, TransportError};
    use crate::messages::MessageRequest;
    use crate::transport::{BodyStream, MockHttpTransport};
    use crate::types::{ApiKey, MaxTokens, ModelId};

    /// Build a single-chunk in-memory `BodyStream` from a byte slice.
    fn body_from_bytes(payload: Vec<u8>) -> BodyStream {
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

    /// Build a `Client<MockHttpTransport>` with the supplied mock.
    fn client_with(transport: MockHttpTransport) -> crate::client::Client<MockHttpTransport> {
        crate::client::Client::builder_with_transport(transport)
            .api_key(ApiKey::new("sk-test").unwrap())
            .build()
            .unwrap()
    }

    fn minimal_request() -> MessageRequest {
        MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .add_user_text("hello")
            .build()
    }

    const HAPPY_MESSAGE: &[u8] = br#"{"id":"msg_01","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"Hi"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":5,"output_tokens":2}}"#;
    const API_ERROR_BODY: &[u8] =
        br#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#;

    // ── create ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_2xx_decodes_message() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(HAPPY_MESSAGE.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let msg = client.messages().create(minimal_request()).await.unwrap();

        assert_eq!(msg.content.len(), 1);
        assert_eq!(msg.usage.input_tokens, 5);
    }

    #[tokio::test]
    async fn create_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(API_ERROR_BODY.to_vec()));
            *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client
            .messages()
            .create(minimal_request())
            .await
            .unwrap_err();

        match err {
            Error::Api(e) => assert_eq!(e.status, StatusCode::TOO_MANY_REQUESTS),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── count_tokens ─────────────────────────────────────────────────────────

    #[cfg(feature = "messages-token-counting")]
    #[tokio::test]
    async fn count_tokens_2xx_decodes_token_count() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let body = br#"{"input_tokens":77}"#;
            let mut resp = Response::new(body_from_bytes(body.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let tc = client
            .messages()
            .count_tokens(minimal_request())
            .await
            .unwrap();

        assert_eq!(tc.input_tokens, 77);
    }

    #[cfg(feature = "messages-token-counting")]
    #[tokio::test]
    async fn count_tokens_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(API_ERROR_BODY.to_vec()));
            *resp.status_mut() = StatusCode::BAD_REQUEST;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client
            .messages()
            .count_tokens(minimal_request())
            .await
            .unwrap_err();

        match err {
            Error::Api(e) => assert_eq!(e.status, StatusCode::BAD_REQUEST),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }
}
