//! `ModelsResource` — the entry point for `GET /models` requests.
//!
//! Exists as its own file so request dispatch is separate from the wire-format
//! type definitions in `model.rs` and the module table of contents. The handle
//! borrows a `Client<T>` and is created at the call site.
//!
//! Responsibilities:
//! - Define [`ModelsResource`], the short-lived handle vended by
//!   [`crate::client::Client::models`].
//! - Implement [`ModelsResource::list`] — build the GET request, dispatch
//!   through the transport, collect the body, and decode the response.
//!
//! Not responsible for:
//! - Body collection and error decoding — those live in `crate::wire_helpers`.
//! - Retry / backoff — handled by a separate layer above this resource.

use bytes::Bytes;
use http::{Method, Request};

use crate::client::Client;
use crate::error::Error;
use crate::headers as h;
use crate::models::model::Model;
use crate::transport::{HttpTransport, MAX_RESPONSE_BODY_BYTES, collect_body};
use crate::wire_helpers::decode_api_error_from_parts;

/// Path joined onto the configured base URL for models-catalog calls.
///
/// No leading slash — relies on the additive-join semantics of `BaseUrl::join`
/// (a base whose path ends with `/`).
const MODELS_PATH: &str = "models";

/// Short-lived handle for the models-catalog endpoint, borrowing a `Client<T>`.
///
/// Obtain via [`Client::models`]; do not construct directly.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "transport-reqwest")]
/// # async fn example() -> Result<(), openrouter_rs::error::Error> {
/// use openrouter_rs::Client;
/// use openrouter_rs::types::ApiKey;
///
/// let client = Client::builder()?
///     .api_key(ApiKey::new("sk-or-v1-...").unwrap())
///     .build()?;
/// let models = client.models().list().await?;
/// println!("catalog has {} models", models.len());
/// # Ok(())
/// # }
/// ```
pub struct ModelsResource<'a, T: HttpTransport> {
    pub(crate) client: &'a Client<T>,
}

/// Private decode wrapper for the `{ "data": [...] }` envelope.
#[derive(serde::Deserialize)]
struct ModelsResponse {
    data: Vec<Model>,
}

impl<T: HttpTransport> ModelsResource<'_, T> {
    /// Fetch the full models catalog and return the list of model entries.
    ///
    /// Issues a `GET /models` request using the client's configured base URL,
    /// authorization, and optional HTTP-referer / app-title headers. Returns
    /// all entries in the `data` array of the response.
    ///
    /// # Errors
    /// - [`Error::Serde`] — a 2xx body cannot be decoded as the models
    ///   response envelope.
    /// - [`Error::Transport`] — a network-level failure occurs while sending
    ///   or reading the response body.
    /// - [`Error::RateLimit`] — the API returns HTTP 429.
    /// - [`Error::Moderation`] — the API returns an HTTP 403 moderation block.
    /// - [`Error::Provider`] — an upstream provider error is passed through.
    /// - [`Error::Api`] — any other non-2xx status with a decodable envelope.
    /// - [`Error::UndecodableApiError`] — a non-2xx status whose body is not a
    ///   recognized error envelope.
    /// - [`Error::InvalidRequest`] — the base URL cannot be joined with the
    ///   models path, or the HTTP request cannot be constructed.
    pub async fn list(&self) -> Result<Vec<Model>, Error> {
        let http_req = self.build_http_request()?;

        let resp = self
            .client
            .inner
            .transport
            .send(http_req)
            .await
            .map_err(Error::Transport)?;

        let (parts, body) = resp.into_parts();
        let body_bytes = collect_body(body, MAX_RESPONSE_BODY_BYTES)
            .await
            .map_err(Error::Transport)?;

        if parts.status.is_success() {
            let envelope = serde_json::from_slice::<ModelsResponse>(&body_bytes).map_err(|e| {
                Error::Serde {
                    context: "ModelsResponse",
                    source: e,
                }
            })?;
            Ok(envelope.data)
        } else {
            Err(decode_api_error_from_parts(
                parts.status,
                &parts.headers,
                &body_bytes,
            ))
        }
    }

    /// Build the `GET /models` HTTP request.
    ///
    /// Sends the `Authorization`, optional `HTTP-Referer`, and optional
    /// `X-Title` headers that the chat path also sends. Uses `Accept:
    /// application/json` and an empty body (no `Content-Type`).
    fn build_http_request(&self) -> Result<Request<Bytes>, Error> {
        let url = self
            .client
            .config()
            .base_url()
            .join(MODELS_PATH)
            .map_err(|e| Error::InvalidRequest(format!("failed to build models URL: {e}")))?;

        let mut builder = Request::builder()
            .method(Method::GET)
            .uri(url.as_str())
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

        builder
            .body(Bytes::new())
            .map_err(|e| Error::InvalidRequest(format!("failed to build HTTP request: {e}")))
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
    use http::{Method, Response, StatusCode};

    use crate::client::Client;
    use crate::error::{Error, TransportError};
    use crate::transport::{BodyStream, MockHttpTransport};
    use crate::types::ApiKey;

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

    const MODELS_RESPONSE: &[u8] = br#"{
        "data": [
            {
                "id": "openai/gpt-4o",
                "name": "OpenAI: GPT-4o",
                "context_length": 128000,
                "pricing": {
                    "prompt": "0.0000025",
                    "completion": "0.00001"
                }
            },
            {
                "id": "anthropic/claude-sonnet-4-5",
                "name": "Anthropic: Claude Sonnet 4.5",
                "context_length": 200000,
                "pricing": {
                    "prompt": "0.000003",
                    "completion": "0.000015",
                    "input_cache_read": "0.0000003"
                }
            }
        ]
    }"#;

    #[tokio::test]
    async fn list_2xx_returns_model_vec() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from(MODELS_RESPONSE.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let models = client.models().list().await.unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id.as_str(), "openai/gpt-4o");
        assert_eq!(models[0].pricing.prompt.as_str(), "0.0000025");
        assert_eq!(models[1].id.as_str(), "anthropic/claude-sonnet-4-5");
        assert_eq!(
            models[1]
                .pricing
                .input_cache_read
                .as_ref()
                .unwrap()
                .as_str(),
            "0.0000003"
        );
    }

    #[tokio::test]
    async fn list_non_2xx_routes_to_error() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let body = br#"{"error":{"code":401,"message":"unauthorized"}}"#.to_vec();
            let mut resp = Response::new(body_from(body));
            *resp.status_mut() = StatusCode::UNAUTHORIZED;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client.models().list().await.unwrap_err();

        match err {
            Error::Api { status, .. } => assert_eq!(status, 401),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_429_routes_to_rate_limit() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let body = br#"{"error":{"code":429,"message":"slow down"}}"#.to_vec();
            let mut resp = Response::new(body_from(body));
            *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            Ok(resp)
        });

        let client = client_with(transport);
        let err = client.models().list().await.unwrap_err();
        assert!(matches!(err, Error::RateLimit { .. }));
    }

    #[tokio::test]
    async fn list_uses_get_method_and_auth_header() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|req| {
            assert_eq!(req.method(), Method::GET);
            let auth = req
                .headers()
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap();
            assert!(auth.starts_with("Bearer "));
            // No Content-Type header on GET
            assert!(req.headers().get("content-type").is_none());
            let mut resp = Response::new(body_from(br#"{"data":[]}"#.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let models = client.models().list().await.unwrap();
        assert!(models.is_empty());
    }

    #[tokio::test]
    async fn list_empty_data_array_returns_empty_vec() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body_from(br#"{"data":[]}"#.to_vec()));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });

        let client = client_with(transport);
        let models = client.models().list().await.unwrap();
        assert!(models.is_empty());
    }
}
