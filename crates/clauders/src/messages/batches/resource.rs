//! `BatchesResource` — HTTP dispatch for the Message Batches API.
//!
//! Exists as its own module so request construction and response decoding
//! are isolated from the type definitions in `types.rs` and the JSONL
//! splitter in `results.rs`.
//!
//! Responsibilities:
//! - Define [`BatchesResource`], the short-lived handle vended by
//!   [`crate::messages::MessagesResource::batches`].
//! - Implement `create`, `get`, `list`, `results`, `cancel`, `delete`.
//! - Decode 2xx bodies into the appropriate types and non-2xx bodies into
//!   [`crate::error::Error`].
//!
//! Not responsible for:
//! - Auth or client configuration — accessed through the borrowed [`Client`].
//! - Body collection or error decoding — shared helpers in
//!   `crate::wire_helpers`.

use bytes::Bytes;
use http::{Method, Request};

use crate::client::Client;
use crate::error::Error;
use crate::headers as h;
use crate::transport::{BodyStream, HttpTransport};
use crate::types::BatchId;
use crate::wire_helpers::{MAX_RESPONSE_BODY_BYTES, collect_body, decode_api_error_from_parts};

use super::results::BatchResultStream;
use super::types::{Batch, BatchList, BatchRequest, DeletedMessageBatch};

/// Path prefix for all batch endpoints.
///
/// No leading slash — relies on `BaseUrl::join` resolution semantics.
const BATCHES_PATH: &str = "v1/messages/batches";

/// Short-lived handle for the Message Batches API, borrowing a `Client<T>`.
///
/// Obtain via [`crate::messages::MessagesResource::batches`]; do not
/// construct directly.
///
/// # Examples
///
/// ```no_run
/// # use clauders::Client;
/// # use clauders::messages::{BatchRequest, MessageRequest};
/// # use clauders::types::{ApiKey, BatchId, CustomRequestId, MaxTokens, ModelId};
/// # async fn example() -> Result<(), clauders::error::Error> {
/// let client = Client::builder()?
///     .api_key(ApiKey::new("sk-ant-…").unwrap())
///     .build()?;
/// let batch_req = BatchRequest::builder()
///     .add(
///         CustomRequestId::new("r1").unwrap(),
///         MessageRequest::builder()
///             .model(ModelId::claude_sonnet_4_5())
///             .max_tokens(MaxTokens::new(16).unwrap())
///             .add_user_text("hi")
///             .build(),
///     )
///     .build();
/// let batch = client.messages().batches().create(batch_req).await?;
/// println!("batch id: {}", batch.id);
/// # Ok(())
/// # }
/// ```
pub struct BatchesResource<'a, T: HttpTransport> {
    pub(crate) client: &'a Client<T>,
}

impl<T: HttpTransport> BatchesResource<'_, T> {
    /// Submit a batch of message requests.
    ///
    /// Sends `POST /v1/messages/batches` and returns the newly created
    /// [`Batch`] object. The batch transitions from `in_progress` to
    /// `ended` asynchronously; poll [`BatchesResource::get`] to check
    /// status.
    ///
    /// # Errors
    /// - [`Error::Serde`] — the request could not be serialized, or the
    ///   2xx response body could not be decoded.
    /// - [`Error::Transport`] — a network-level failure occurred.
    /// - [`Error::Api`] — the API returned a non-2xx status.
    /// - [`Error::UndecodableApiError`] — non-2xx body is not a recognized
    ///   error envelope.
    /// - [`Error::InvalidRequest`] — the base URL join failed, or HTTP
    ///   request construction failed.
    pub async fn create(&self, req: BatchRequest) -> Result<Batch, Error> {
        let body = serde_json::to_vec(&req).map_err(|e| Error::Serde {
            context: "BatchRequest",
            source: e,
        })?;
        let resp = self
            .send(Method::POST, BATCHES_PATH, Bytes::from(body), false)
            .await?;
        decode_into::<Batch>(resp, "Batch").await
    }

    /// Retrieve the current status of a batch.
    ///
    /// Sends `GET /v1/messages/batches/{id}` and returns the [`Batch`]
    /// object.
    ///
    /// # Errors
    /// Same as [`BatchesResource::create`].
    pub async fn get(&self, id: &BatchId) -> Result<Batch, Error> {
        let path = format!("{BATCHES_PATH}/{}", id.as_str());
        let resp = self.send(Method::GET, &path, Bytes::new(), false).await?;
        decode_into::<Batch>(resp, "Batch").await
    }

    /// List batches, most-recently created first.
    ///
    /// Sends `GET /v1/messages/batches` and returns one page of results.
    /// Use `BatchList::last_id` as a cursor for subsequent requests.
    ///
    /// # Errors
    /// Same as [`BatchesResource::create`].
    pub async fn list(&self) -> Result<BatchList, Error> {
        let resp = self
            .send(Method::GET, BATCHES_PATH, Bytes::new(), false)
            .await?;
        decode_into::<BatchList>(resp, "BatchList").await
    }

    /// Stream the JSONL results for an ended batch.
    ///
    /// Sends `GET /v1/messages/batches/{id}/results` and returns a
    /// [`BatchResultStream`] that yields one decoded [`super::types::BatchResultRow`]
    /// per line. The response status is checked before the stream is returned;
    /// only a 2xx response yields the stream handle.
    ///
    /// # Errors
    /// Same as [`BatchesResource::create`], plus:
    /// - [`Error::JsonLines`] — a JSONL line could not be decoded (yielded
    ///   by the returned stream, not by this call).
    pub async fn results(&self, id: &BatchId) -> Result<BatchResultStream, Error> {
        let path = format!("{BATCHES_PATH}/{}/results", id.as_str());
        let resp = self.send(Method::GET, &path, Bytes::new(), true).await?;
        let (parts, body) = resp.into_parts();
        if !parts.status.is_success() {
            let bytes = collect_body(body, MAX_RESPONSE_BODY_BYTES)
                .await
                .map_err(Error::Transport)?;
            return Err(decode_api_error_from_parts(
                parts.status,
                &parts.headers,
                &bytes,
            ));
        }
        Ok(BatchResultStream::new(body))
    }

    /// Cancel a batch that is still in progress.
    ///
    /// Sends `POST /v1/messages/batches/{id}/cancel` and returns the
    /// updated [`Batch`] object reflecting the cancellation.
    ///
    /// # Errors
    /// Same as [`BatchesResource::create`].
    pub async fn cancel(&self, id: &BatchId) -> Result<Batch, Error> {
        let path = format!("{BATCHES_PATH}/{}/cancel", id.as_str());
        let resp = self.send(Method::POST, &path, Bytes::new(), false).await?;
        decode_into::<Batch>(resp, "Batch").await
    }

    /// Delete a batch.
    ///
    /// Sends `DELETE /v1/messages/batches/{id}` and returns a
    /// [`DeletedMessageBatch`] confirming the deletion.
    ///
    /// # Errors
    /// Same as [`BatchesResource::create`].
    pub async fn delete(&self, id: &BatchId) -> Result<DeletedMessageBatch, Error> {
        let path = format!("{BATCHES_PATH}/{}", id.as_str());
        let resp = self
            .send(Method::DELETE, &path, Bytes::new(), false)
            .await?;
        decode_into::<DeletedMessageBatch>(resp, "DeletedMessageBatch").await
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        body: Bytes,
        is_jsonl: bool,
    ) -> Result<http::Response<BodyStream>, Error> {
        let url = self
            .client
            .config()
            .base_url()
            .join(path)
            .map_err(|e| Error::InvalidRequest(format!("base URL join failed: {e}")))?;

        let accept = if is_jsonl {
            "application/x-jsonl"
        } else {
            h::APPLICATION_JSON
        };

        let mut builder = Request::builder()
            .method(method)
            .uri(url.as_str())
            .header(h::CONTENT_TYPE, h::APPLICATION_JSON)
            .header(h::ACCEPT, accept)
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

        let req = builder
            .body(body)
            .map_err(|e| Error::InvalidRequest(format!("http::Request::body: {e}")))?;

        self.client
            .inner
            .transport
            .send(req)
            .await
            .map_err(Error::Transport)
    }
}

async fn decode_into<U>(resp: http::Response<BodyStream>, context: &'static str) -> Result<U, Error>
where
    U: for<'de> serde::Deserialize<'de>,
{
    let (parts, body) = resp.into_parts();
    let bytes = collect_body(body, MAX_RESPONSE_BODY_BYTES)
        .await
        .map_err(Error::Transport)?;
    if parts.status.is_success() {
        serde_json::from_slice::<U>(&bytes).map_err(|e| Error::Serde { context, source: e })
    } else {
        Err(decode_api_error_from_parts(
            parts.status,
            &parts.headers,
            &bytes,
        ))
    }
}

#[cfg(all(test, feature = "__test-mocks"))]
mod tests {
    //! Tests for the decode branches and URL construction in
    //! `BatchesResource`. Covers 2xx success, non-2xx error, and
    //! path-segment construction for each operation.

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
    use crate::messages::request::MessageRequest;
    use crate::transport::{BodyStream, MockHttpTransport};
    use crate::types::{ApiKey, BaseUrl, BatchId, CustomRequestId, MaxTokens, ModelId};

    use super::super::types::{BatchRequest, BatchStatus};

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

    fn client_with(transport: MockHttpTransport) -> crate::client::Client<MockHttpTransport> {
        crate::client::Client::builder_with_transport(transport)
            .api_key(ApiKey::new("sk-test").unwrap())
            .build()
            .unwrap()
    }

    fn client_with_base(
        transport: MockHttpTransport,
        base: &str,
    ) -> crate::client::Client<MockHttpTransport> {
        crate::client::Client::builder_with_transport(transport)
            .api_key(ApiKey::new("sk-test").unwrap())
            .base_url(BaseUrl::parse(base).unwrap())
            .build()
            .unwrap()
    }

    fn minimal_request() -> MessageRequest {
        MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(8).unwrap())
            .add_user_text("hi")
            .build()
    }

    const BATCH_JSON: &[u8] = br#"{"id":"msgbatch_01","type":"message_batch","processing_status":"in_progress","request_counts":{"processing":2,"succeeded":0,"errored":0,"canceled":0,"expired":0},"ended_at":null,"created_at":"2026-05-28T00:00:00Z","expires_at":"2026-05-29T00:00:00Z","archived_at":null,"cancel_initiated_at":null,"results_url":null}"#;
    const API_ERROR_BODY: &[u8] =
        br#"{"type":"error","error":{"type":"invalid_request_error","message":"bad"}}"#;

    // ── create 2xx ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_2xx_decodes_batch() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(BATCH_JSON.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let batch = client
            .messages()
            .batches()
            .create(
                BatchRequest::builder()
                    .add(CustomRequestId::new("r1").unwrap(), minimal_request())
                    .build(),
            )
            .await
            .unwrap();

        assert_eq!(batch.id.as_str(), "msgbatch_01");
        assert_eq!(batch.processing_status, BatchStatus::InProgress);
    }

    // ── create non-2xx ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(API_ERROR_BODY.to_vec()));
            *resp.status_mut() = StatusCode::BAD_REQUEST;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client
            .messages()
            .batches()
            .create(BatchRequest::builder().build())
            .await
            .unwrap_err();

        match err {
            Error::Api(e) => assert_eq!(e.status, StatusCode::BAD_REQUEST),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── get 2xx ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_2xx_decodes_batch() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(BATCH_JSON.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let id = BatchId::new("msgbatch_01").unwrap();
        let batch = client.messages().batches().get(&id).await.unwrap();
        assert_eq!(batch.id.as_str(), "msgbatch_01");
    }

    // ── get non-2xx ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(API_ERROR_BODY.to_vec()));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        });

        let client = client_with(transport);
        let id = BatchId::new("msgbatch_01").unwrap();
        let err = client.messages().batches().get(&id).await.unwrap_err();

        match err {
            Error::Api(e) => assert_eq!(e.status, StatusCode::NOT_FOUND),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── list 2xx ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_2xx_decodes_batch_list() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let body = br#"{"data":[],"has_more":false,"first_id":null,"last_id":null}"#;
            let mut resp = Response::new(body_from_bytes(body.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let list = client.messages().batches().list().await.unwrap();
        assert!(!list.has_more);
        assert!(list.data.is_empty());
    }

    // ── results non-2xx ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn results_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(API_ERROR_BODY.to_vec()));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        });

        let client = client_with(transport);
        let id = BatchId::new("msgbatch_01").unwrap();
        let err = client.messages().batches().results(&id).await.unwrap_err();

        match err {
            Error::Api(e) => assert_eq!(e.status, StatusCode::NOT_FOUND),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── delete 200 + body ───────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_200_returns_deleted_batch() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let body = br#"{"id":"msgbatch_01","type":"message_batch_deleted"}"#;
            let mut resp = Response::new(body_from_bytes(body.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let id = BatchId::new("msgbatch_01").unwrap();
        let deleted = client.messages().batches().delete(&id).await.unwrap();
        assert_eq!(deleted.id.as_str(), "msgbatch_01");
    }

    // ── delete non-2xx ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(API_ERROR_BODY.to_vec()));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        });

        let client = client_with(transport);
        let id = BatchId::new("msgbatch_01").unwrap();
        let err = client.messages().batches().delete(&id).await.unwrap_err();

        match err {
            Error::Api(e) => assert_eq!(e.status, StatusCode::NOT_FOUND),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── path construction ───────────────────────────────────────────────────

    #[tokio::test]
    async fn results_uri_contains_id_and_results_segment() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|req| {
            let uri = req.uri().to_string();
            assert!(
                uri.contains("v1/messages/batches/msgbatch_01/results"),
                "expected URI to contain batch results path, got: {uri}"
            );
            let mut resp = Response::new(body_from_bytes(b"".to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with_base(transport, "https://api.anthropic.com");
        let id = BatchId::new("msgbatch_01").unwrap();
        // The empty stream will just return None immediately.
        let _ = client.messages().batches().results(&id).await.unwrap();
    }
}
