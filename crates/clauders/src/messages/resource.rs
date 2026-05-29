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
//! - Implement `parse_retry_after` and `collect_body` as internal helpers.
//!
//! Not responsible for:
//! - Retry logic — the client layer owns that.
//! - Streaming responses — a separate variant in a future extension.
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

            match serde_json::from_slice::<ApiErrorEnvelope>(&body_bytes) {
                Ok(envelope) => Err(Error::Api(ApiError {
                    status: parts.status,
                    body: envelope.error,
                    request_id,
                    organization_id,
                    retry_after,
                })),
                Err(_) => Err(Error::UndecodableApiError {
                    status: parts.status,
                    detail: String::from_utf8_lossy(&body_bytes).into_owned(),
                    request_id,
                }),
            }
        }
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

    use super::*;

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
