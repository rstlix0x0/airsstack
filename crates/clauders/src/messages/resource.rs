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

/// Models for which `thinking.type = "enabled"` is deprecated in favour of
/// `"adaptive"`. Matches the list both official SDKs carry.
const MODELS_TO_WARN_WITH_THINKING_ENABLED: [&str; 2] =
    ["claude-opus-4-6", "claude-mythos-preview"];

/// Whether this request pairs a listed model with `thinking.type = "enabled"`.
///
/// Split from the emitter so the condition is unit-testable without
/// capturing log output.
fn should_warn_deprecated_thinking(req: &MessageRequest) -> bool {
    MODELS_TO_WARN_WITH_THINKING_ENABLED.contains(&req.model.as_str())
        && matches!(
            req.thinking,
            Some(crate::messages::thinking::ThinkingConfig::Enabled { .. })
        )
}

/// Emit the deprecation warning when [`should_warn_deprecated_thinking`] holds.
fn warn_if_deprecated_thinking(req: &MessageRequest) {
    if should_warn_deprecated_thinking(req) {
        tracing::warn!(
            model = req.model.as_str(),
            "Using Claude with this model and 'thinking.type=enabled' is deprecated. \
             Use 'thinking.type=adaptive' instead which results in better model \
             performance in our testing: \
             https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking"
        );
    }
}

/// Model → end-of-life date. Verbatim copy of the table both official SDKs
/// carry. The value is a display string, not a parsed date, so the emitted
/// warning text matches the SDKs byte-for-byte. Hand-maintained: it drifts
/// from the upstream list until manually updated, exactly as the SDK tables
/// do. Alias pairs (e.g. `-latest` / dated) are both listed on purpose — the
/// lookup matches the literal model string a caller sends.
const DEPRECATED_MODELS: &[(&str, &str)] = &[
    ("claude-1.3", "November 6th, 2024"),
    ("claude-1.3-100k", "November 6th, 2024"),
    ("claude-instant-1.1", "November 6th, 2024"),
    ("claude-instant-1.1-100k", "November 6th, 2024"),
    ("claude-instant-1.2", "November 6th, 2024"),
    ("claude-3-sonnet-20240229", "July 21st, 2025"),
    ("claude-3-opus-20240229", "January 5th, 2026"),
    ("claude-2.1", "July 21st, 2025"),
    ("claude-2.0", "July 21st, 2025"),
    ("claude-3-7-sonnet-latest", "February 19th, 2026"),
    ("claude-3-7-sonnet-20250219", "February 19th, 2026"),
    ("claude-3-5-haiku-latest", "February 19th, 2026"),
    ("claude-3-5-haiku-20241022", "February 19th, 2026"),
    ("claude-opus-4-0", "June 15th, 2026"),
    ("claude-opus-4-20250514", "June 15th, 2026"),
    ("claude-sonnet-4-0", "June 15th, 2026"),
    ("claude-sonnet-4-20250514", "June 15th, 2026"),
    ("claude-opus-4-1", "August 5th, 2026"),
    ("claude-opus-4-1-20250805", "August 5th, 2026"),
    ("claude-mythos-preview", "June 30th, 2026"),
];

/// The end-of-life date if `req`'s model is deprecated, else `None`.
///
/// Split from the emitter so the decision is unit-testable without capturing
/// log output, mirroring [`should_warn_deprecated_thinking`].
fn deprecated_model_eol(req: &MessageRequest) -> Option<&'static str> {
    DEPRECATED_MODELS
        .iter()
        .find(|(model, _)| *model == req.model.as_str())
        .map(|(_, date)| *date)
}

/// Emit the deprecation warning when the request names a deprecated model.
fn warn_if_deprecated_model(req: &MessageRequest) {
    if let Some(date) = deprecated_model_eol(req) {
        let model = req.model.as_str();
        tracing::warn!(
            model,
            "The model '{model}' is deprecated and will reach end-of-life on \
             {date}.\nPlease migrate to a newer model. Visit \
             https://docs.anthropic.com/en/docs/resources/model-deprecations for \
             more information."
        );
    }
}

/// Short-lived handle for the Messages API, borrowing a `Client<T>`.
///
/// Obtain via [`Client::messages`]; do not construct directly.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), clauders::error::Error> {
/// # use clauders::Client;
/// # use clauders::messages::MessageRequest;
/// # use clauders::types::{ApiKey, MaxTokens, ModelId};
/// let client = Client::builder()?
///     .api_key(ApiKey::new("sk-ant-…").unwrap())
///     .build()?;
/// let req = MessageRequest::builder()
///     .model(ModelId::claude_sonnet_5())
///     .max_tokens(MaxTokens::new(1024))
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
        warn_if_deprecated_thinking(&req);
        warn_if_deprecated_model(&req);
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
    pub async fn stream(
        &self,
        mut req: MessageRequest,
    ) -> Result<super::streaming::MessageStream, Error> {
        warn_if_deprecated_thinking(&req);
        warn_if_deprecated_model(&req);
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

#[cfg(test)]
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
    use crate::test_support::MockHttpTransport;
    use crate::transport::BodyStream;
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
            .max_tokens(MaxTokens::new(64))
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

    // `create_with_unserializable_content_surfaces_typed_serde_error` was removed
    // with the content-block union split: a request now carries `ContentBlockParam`,
    // which has no `Unknown` arm, so an unserializable request block can no longer be
    // constructed. The failure it exercised is now a compile error, not a runtime one.

    // ── count_tokens ─────────────────────────────────────────────────────────

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

    // ── deprecated thinking warning ─────────────────────────────────────────

    use super::{deprecated_model_eol, should_warn_deprecated_thinking};
    use crate::messages::thinking::ThinkingConfig;

    // `ModelId::custom` is the fallible free-form constructor; the crate has
    // no `ModelId::new`. The named constructors (`claude_sonnet_4_5` etc.)
    // do not cover the two deprecated models this test needs.
    fn req(model: &str, thinking: Option<ThinkingConfig>) -> MessageRequest {
        let b = MessageRequest::builder()
            .model(ModelId::custom(model).unwrap())
            .max_tokens(MaxTokens::new(64))
            .add_user_text("Hi");
        match thinking {
            Some(t) => b.thinking(t).build(),
            None => b.build(),
        }
    }

    #[test]
    fn warns_for_enabled_thinking_on_a_listed_model() {
        assert!(should_warn_deprecated_thinking(&req(
            "claude-opus-4-6",
            Some(ThinkingConfig::enabled(1024))
        )));
        assert!(should_warn_deprecated_thinking(&req(
            "claude-mythos-preview",
            Some(ThinkingConfig::enabled(1024))
        )));
    }

    #[test]
    fn does_not_warn_for_adaptive_or_disabled_on_a_listed_model() {
        assert!(!should_warn_deprecated_thinking(&req(
            "claude-opus-4-6",
            Some(ThinkingConfig::adaptive())
        )));
        assert!(!should_warn_deprecated_thinking(&req(
            "claude-opus-4-6",
            Some(ThinkingConfig::disabled())
        )));
    }

    #[test]
    fn does_not_warn_when_thinking_is_unset() {
        assert!(!should_warn_deprecated_thinking(&req(
            "claude-opus-4-6",
            None
        )));
    }

    #[test]
    fn does_not_warn_for_an_unlisted_model() {
        assert!(!should_warn_deprecated_thinking(&req(
            "claude-sonnet-4-5",
            Some(ThinkingConfig::enabled(1024))
        )));
    }

    // ── deprecated model warning ────────────────────────────────────────────

    #[test]
    fn eol_for_a_listed_model_returns_its_exact_date() {
        assert_eq!(
            deprecated_model_eol(&req("claude-opus-4-1", None)),
            Some("August 5th, 2026")
        );
    }

    #[test]
    fn eol_for_a_second_listed_model_returns_its_own_date() {
        // guards against a single hard-coded return value
        assert_eq!(
            deprecated_model_eol(&req("claude-3-opus-20240229", None)),
            Some("January 5th, 2026")
        );
    }

    #[test]
    fn eol_for_a_current_model_returns_none() {
        assert_eq!(deprecated_model_eol(&req("claude-sonnet-5", None)), None);
    }

    #[test]
    fn deprecated_and_thinking_warnings_are_independent() {
        // `claude-mythos-preview` is in BOTH tables; both predicates must fire
        // so wiring one warning never suppresses the other.
        let r = req("claude-mythos-preview", Some(ThinkingConfig::enabled(1024)));
        assert_eq!(deprecated_model_eol(&r), Some("June 30th, 2026"));
        assert!(should_warn_deprecated_thinking(&r));
    }
}
