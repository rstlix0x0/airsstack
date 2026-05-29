//! `ModelsResource` — handle for `GET /v1/models` and `GET /v1/models/{id}`.
//!
//! Exists as its own module so request dispatch is separate from the
//! type definitions in `types.rs` and the module table of contents in
//! `mod.rs`.
//!
//! Responsibilities:
//! - Define [`ModelsResource`], the short-lived handle vended by
//!   [`crate::client::Client::models`].
//! - Implement `list` — send `GET /v1/models` and decode a [`ModelList`].
//! - Implement `get` — send `GET /v1/models/{id}` and decode a [`ModelInfo`].
//!
//! Not responsible for:
//! - Auth or config — accessed through the borrowed [`Client`].
//! - Body collection or error decoding — those are in a shared
//!   internal helper module used across resource modules.

use bytes::Bytes;
use http::{Method, Request};

use crate::client::Client;
use crate::error::Error;
use crate::headers as h;
use crate::transport::{BodyStream, HttpTransport};
use crate::types::ModelId;
use crate::wire_helpers::{MAX_RESPONSE_BODY_BYTES, collect_body, decode_api_error_from_parts};

use super::types::{ModelInfo, ModelList};

/// Short-lived handle for the Models API, borrowing a `Client<T>`.
///
/// Obtain via [`Client::models`]; do not construct directly.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "transport-reqwest")]
/// # async fn example() -> Result<(), clauders::error::Error> {
/// # use clauders::Client;
/// # use clauders::types::{ApiKey, ModelId};
/// let client = Client::builder()?
///     .api_key(ApiKey::new("sk-ant-…").unwrap())
///     .build()?;
/// let list = client.models().list().await?;
/// println!("{} models available", list.data.len());
/// # Ok(())
/// # }
/// ```
pub struct ModelsResource<'a, T: HttpTransport> {
    pub(crate) client: &'a Client<T>,
}

impl<T: HttpTransport> ModelsResource<'_, T> {
    /// List all available Claude models.
    ///
    /// Sends `GET /v1/models` and returns a [`ModelList`] containing the
    /// current page of results.
    ///
    /// # Errors
    /// - [`Error::Serde`] — the response cannot be decoded as a
    ///   [`ModelList`].
    /// - [`Error::Transport`] — a network-level failure occurs.
    /// - [`Error::Api`] — the API returns a non-2xx status with a decodable
    ///   error envelope.
    /// - [`Error::UndecodableApiError`] — the API returns a non-2xx status
    ///   whose body cannot be parsed.
    /// - [`Error::InvalidRequest`] — the base URL cannot be joined with the
    ///   models path.
    pub async fn list(&self) -> Result<ModelList, Error> {
        let url = self
            .client
            .config()
            .base_url()
            .join("v1/models")
            .map_err(|e| Error::InvalidRequest(format!("failed to build models URL: {e}")))?;

        let resp = self.send_get(url.as_str()).await?;
        self.decode_into::<ModelList>(resp, "ModelList").await
    }

    /// Retrieve metadata for a specific Claude model by ID.
    ///
    /// Sends `GET /v1/models/{id}` and returns a single [`ModelInfo`].
    ///
    /// # Errors
    /// - [`Error::Serde`] — the response cannot be decoded as a
    ///   [`ModelInfo`].
    /// - [`Error::Transport`] — a network-level failure occurs.
    /// - [`Error::Api`] — the API returns a non-2xx status with a decodable
    ///   error envelope.
    /// - [`Error::UndecodableApiError`] — the API returns a non-2xx status
    ///   whose body cannot be parsed.
    /// - [`Error::InvalidRequest`] — the base URL cannot be joined with the
    ///   model path.
    pub async fn get(&self, id: &ModelId) -> Result<ModelInfo, Error> {
        let segment = format!("v1/models/{}", id.as_str());
        let url = self
            .client
            .config()
            .base_url()
            .join(&segment)
            .map_err(|e| Error::InvalidRequest(format!("failed to build model URL: {e}")))?;

        let resp = self.send_get(url.as_str()).await?;
        self.decode_into::<ModelInfo>(resp, "ModelInfo").await
    }

    async fn send_get(&self, url: &str) -> Result<http::Response<BodyStream>, Error> {
        let mut builder = Request::builder()
            .method(Method::GET)
            .uri(url)
            .header(h::ACCEPT, h::APPLICATION_JSON)
            .header(
                h::ANTHROPIC_VERSION,
                self.client.config().anthropic_version().as_str(),
            );

        if let Some(key) = self.client.auth().api_key() {
            builder = builder.header(h::X_API_KEY, key.expose_secret());
        }

        let http_req = builder
            .body(Bytes::new())
            .map_err(|e| Error::InvalidRequest(format!("failed to build HTTP request: {e}")))?;

        self.client
            .inner
            .transport
            .send(http_req)
            .await
            .map_err(Error::Transport)
    }

    async fn decode_into<U>(
        &self,
        resp: http::Response<BodyStream>,
        context: &'static str,
    ) -> Result<U, Error>
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
}

#[cfg(all(test, feature = "__test-mocks"))]
mod tests {
    //! Tests for the decode branches and URL construction in
    //! `ModelsResource`. The body-collection and error-decoding helpers are
    //! tested in `crate::wire_helpers::tests`; these tests focus on the
    //! status-code branch and the `get` path segment.

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
    use crate::transport::{BodyStream, MockHttpTransport};
    use crate::types::{ApiKey, BaseUrl, ModelId};

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

    /// Build a client whose base URL matches `base` so we can assert on the
    /// full request URI in the mock expectation closure.
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

    const MODEL_LIST_BODY: &[u8] =
        br#"{"data":[],"has_more":false,"first_id":null,"last_id":null}"#;
    const MODEL_INFO_BODY: &[u8] = br#"{"id":"claude-sonnet-4-5","display_name":"Claude Sonnet 4.5","created_at":"2025-09-01T00:00:00Z","type":"model"}"#;
    const API_ERROR_BODY: &[u8] =
        br#"{"type":"error","error":{"type":"permission_error","message":"not found"}}"#;

    // ── decode_into success ───────────────────────────────────────────────────

    #[tokio::test]
    async fn list_2xx_decodes_model_list() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(MODEL_LIST_BODY.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let list = client.models().list().await.unwrap();
        assert!(!list.has_more);
        assert!(list.data.is_empty());
    }

    // ── decode_into error branch ──────────────────────────────────────────────

    #[tokio::test]
    async fn list_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(API_ERROR_BODY.to_vec()));
            *resp.status_mut() = StatusCode::FORBIDDEN;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client.models().list().await.unwrap_err();

        match err {
            Error::Api(e) => assert_eq!(e.status, StatusCode::FORBIDDEN),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── get builds v1/models/{id} path ────────────────────────────────────────

    #[tokio::test]
    async fn get_builds_correct_path_for_model_id() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|req| {
            // The URI must contain the model ID segment.
            let uri = req.uri().to_string();
            assert!(
                uri.contains("v1/models/claude-sonnet-4-5"),
                "expected URI to contain `v1/models/claude-sonnet-4-5`, got `{uri}`"
            );
            let mut resp = Response::new(body_from_bytes(MODEL_INFO_BODY.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with_base(transport, "https://api.anthropic.com");
        let info = client
            .models()
            .get(&ModelId::claude_sonnet_4_5())
            .await
            .unwrap();

        assert_eq!(info.display_name, "Claude Sonnet 4.5");
    }

    #[tokio::test]
    async fn get_non_2xx_returns_api_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from_bytes(API_ERROR_BODY.to_vec()));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client
            .models()
            .get(&ModelId::claude_sonnet_4_5())
            .await
            .unwrap_err();

        match err {
            Error::Api(e) => assert_eq!(e.status, StatusCode::NOT_FOUND),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }
}
