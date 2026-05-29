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
//! - Implement `parse_retry_after`, `collect_body`, and `decode_error_body`
//!   as internal helpers shared across `create` and `stream`.
//!
//! Not responsible for:
//! - Retry logic — the client layer owns that.
//! - Auth schemes beyond API-key — `Auth` handles that.

use std::time::Duration;

use bytes::Bytes;
use http::{Method, Request};

use crate::client::Client;
use crate::error::{ApiError, ApiErrorBody, Error, TransportError};
use crate::headers as h;
use crate::transport::{BodyStream, HttpTransport};
use crate::types::{OrganizationId, RequestId};

use super::request::MessageRequest;
use super::response::Message;

/// Path appended to the configured base URL for Messages API calls.
///
/// Value: `v1/messages` (no leading slash — relies on `BaseUrl::join`
/// segment-resolution semantics documented on that method).
const MESSAGES_PATH: &str = "v1/messages";

/// Maximum response body size accepted before truncation.
///
/// 16 MiB matches a conservative ceiling well above any plausible
/// non-streaming response from the Anthropic API.
const MAX_RESPONSE_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Short-lived handle for the Messages API, borrowing a `Client<T>`.
///
/// Obtain via [`Client::messages`]; do not construct directly.
///
/// # Examples
///
/// ```no_run
/// # use clauders::Client;
/// # use clauders::messages::MessageRequest;
/// # use clauders::types::{ApiKey, MaxTokens, ModelId};
/// # async fn example() -> Result<(), clauders::error::Error> {
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

        let request_id = parts
            .headers
            .get(h::REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| RequestId::new(s).ok());

        let organization_id = parts
            .headers
            .get(h::ANTHROPIC_ORG_ID)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| OrganizationId::new(s).ok());

        let body_bytes = collect_body(body, MAX_RESPONSE_BODY_BYTES)
            .await
            .map_err(Error::Transport)?;

        if parts.status.is_success() {
            serde_json::from_slice::<Message>(&body_bytes).map_err(|e| Error::Serde {
                context: "Message",
                source: e,
            })
        } else {
            let retry_after = parts
                .headers
                .get(h::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_retry_after);

            Err(decode_error_body(
                parts.status,
                &body_bytes,
                request_id,
                organization_id,
                retry_after,
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

            let request_id = parts
                .headers
                .get(h::REQUEST_ID)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| RequestId::new(s).ok());

            let organization_id = parts
                .headers
                .get(h::ANTHROPIC_ORG_ID)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| OrganizationId::new(s).ok());

            let retry_after = parts
                .headers
                .get(h::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_retry_after);

            return Err(decode_error_body(
                parts.status,
                &body_bytes,
                request_id,
                organization_id,
                retry_after,
            ));
        }

        Ok(super::streaming::MessageStream::new(body_stream))
    }
}

/// Outer error envelope the Anthropic API wraps every non-2xx body in.
///
/// Wire format: `{"type":"error","error":{...}}`. The outer `"type":"error"`
/// field is consumed here; the inner object maps to [`ApiErrorBody`].
#[derive(serde::Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

/// Decode a non-2xx response body into an [`Error`].
///
/// Attempts to parse the body as an [`ApiErrorEnvelope`]; falls back to
/// [`Error::UndecodableApiError`] when the body is not a recognized envelope.
fn decode_error_body(
    status: http::StatusCode,
    body_bytes: &[u8],
    request_id: Option<RequestId>,
    organization_id: Option<OrganizationId>,
    retry_after: Option<std::time::Duration>,
) -> Error {
    match serde_json::from_slice::<ApiErrorEnvelope>(body_bytes) {
        Ok(envelope) => Error::Api(ApiError {
            status,
            body: envelope.error,
            request_id,
            organization_id,
            retry_after,
        }),
        Err(_) => Error::UndecodableApiError {
            status,
            detail: String::from_utf8_lossy(body_bytes).into_owned(),
            request_id,
        },
    }
}

/// Parse a `Retry-After` header value as an integer number of seconds.
///
/// Returns `None` when the value is not a non-negative integer (e.g. an
/// HTTP-date format, which is not returned by the current Anthropic API).
fn parse_retry_after(value: &str) -> Option<Duration> {
    value.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// Collect a [`BodyStream`] into a `Vec<u8>`, stopping at `limit` bytes.
///
/// Returns [`TransportError::BodyStream`] if the stream yields an error
/// or if the accumulated size exceeds `limit`.
async fn collect_body(mut stream: BodyStream, limit: usize) -> Result<Vec<u8>, TransportError> {
    let mut buf = Vec::new();
    loop {
        // Drive the stream with std::future::poll_fn to avoid a futures-util dep.
        let item = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await;
        match item {
            None => break,
            Some(Err(e)) => return Err(e),
            Some(Ok(chunk)) => {
                if buf.len() + chunk.len() > limit {
                    return Err(TransportError::BodyStream(format!(
                        "response body exceeded {limit} byte limit"
                    )));
                }
                buf.extend_from_slice(&chunk);
            }
        }
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::panic,
        reason = "test-only panics on wrong-variant matches; a panic is the intended failure signal"
    )]

    use super::*;

    // ── decode_error_body ──────────────────────────────────────────────────────

    #[test]
    fn decode_error_body_valid_envelope_produces_api_error() {
        use crate::error::{ApiError, ErrorType};
        use crate::types::{OrganizationId, RequestId};
        use http::StatusCode;
        use std::time::Duration;

        let body = br#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#;
        let request_id = Some(RequestId::new("req_abc").unwrap());
        let organization_id = Some(OrganizationId::new("org_xyz").unwrap());
        let retry_after = Some(Duration::from_secs(5));

        let err = decode_error_body(
            StatusCode::TOO_MANY_REQUESTS,
            body,
            request_id,
            organization_id,
            retry_after,
        );

        match err {
            Error::Api(ApiError {
                status,
                body: error_body,
                request_id: rid,
                organization_id: oid,
                retry_after: ra,
            }) => {
                assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
                assert_eq!(error_body.kind, ErrorType::RateLimitError);
                assert_eq!(error_body.message, "slow down");
                assert_eq!(rid.as_ref().map(RequestId::as_str), Some("req_abc"));
                assert_eq!(oid.as_ref().map(OrganizationId::as_str), Some("org_xyz"));
                assert_eq!(ra, Some(Duration::from_secs(5)));
            }
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    #[test]
    fn decode_error_body_garbage_body_produces_undecodable_api_error() {
        use crate::types::RequestId;
        use http::StatusCode;

        let body = b"this is not json at all";
        let request_id = Some(RequestId::new("req_123").unwrap());

        let err = decode_error_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            body,
            request_id,
            None,
            None,
        );

        match err {
            Error::UndecodableApiError {
                status,
                detail,
                request_id: rid,
            } => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(detail, "this is not json at all");
                assert_eq!(rid.as_ref().map(RequestId::as_str), Some("req_123"));
            }
            other => panic!("expected Error::UndecodableApiError, got {other:?}"),
        }
    }

    #[test]
    fn parse_retry_after_parses_integer_seconds() {
        let d = parse_retry_after("30").unwrap();
        assert_eq!(d, std::time::Duration::from_secs(30));
    }

    #[test]
    fn parse_retry_after_returns_none_for_non_integer() {
        assert!(parse_retry_after("not-a-number").is_none());
        assert!(parse_retry_after("").is_none());
    }

    #[test]
    fn collect_body_up_to_limit() {
        use bytes::Bytes;
        use futures_core::Stream;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        struct OneShotStream(Option<Bytes>);

        impl Stream for OneShotStream {
            type Item = Result<Bytes, crate::error::TransportError>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                Poll::Ready(self.0.take().map(Ok))
            }
        }

        let stream: BodyStream = Box::pin(OneShotStream(Some(Bytes::from("hello world"))));
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let result = rt.block_on(collect_body(stream, 1024)).unwrap();
        assert_eq!(result, b"hello world");
    }
}
