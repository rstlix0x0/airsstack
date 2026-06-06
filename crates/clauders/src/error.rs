//! Error types for the SDK.
//!
//! The error surface is layered so callers can match on the failure
//! domain without parsing strings:
//!
//! - [`TransportError`] — failures in the HTTP transport layer (network,
//!   TLS, timeout, body framing).
//! - [`ApiError`] — non-2xx API responses with a decoded error envelope.
//! - [`BuildError`] — client construction failures detectable before any request.
//! - [`Error`] — top-level wrapper; every fallible public SDK call returns
//!   `Result<T, Error>`.

use std::time::Duration;

pub use airs_transport::TransportError;

use crate::types::{OrganizationId, RequestId};
use http::StatusCode;

/// API-layer error: a non-2xx response from the Anthropic API whose
/// envelope (`{ "type": "error", "error": { ... } }`) was successfully
/// decoded.
///
/// Preserves the protocol-level metadata the server returns regardless
/// of body-decode success: `request_id` (echo header), `organization_id`
/// (echo header), and `retry_after` when supplied on 429/529.
///
/// Use [`ApiError::is_retryable`] to decide whether a request is safe
/// to retry. Rate-limit and overloaded responses are retryable; the
/// `Retry-After` header value is preserved on [`ApiError::retry_after`].
#[derive(Debug, Clone, thiserror::Error)]
#[error("API error {status} ({}): {}", body.kind, body.message)]
#[non_exhaustive]
pub struct ApiError {
    /// HTTP status code returned by the API.
    pub status: StatusCode,
    /// Decoded error envelope body.
    pub body: ApiErrorBody,
    /// `request-id` response header value, if present.
    pub request_id: Option<RequestId>,
    /// `anthropic-organization-id` response header value, if present.
    pub organization_id: Option<OrganizationId>,
    /// `Retry-After` header value, parsed as a `Duration`, if present.
    pub retry_after: Option<Duration>,
}

/// Decoded inner object of an Anthropic error envelope.
///
/// The wire format is `{ "type": "error", "error": { "type": "...", "message": "..." } }`.
/// This struct represents the inner object — the outer `type: "error"`
/// discriminator is consumed by the transport layer before constructing
/// the error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ApiErrorBody {
    /// Anthropic error category — see [`ErrorType`].
    #[serde(rename = "type")]
    pub kind: ErrorType,
    /// Human-readable error message from the API.
    pub message: String,
}

/// Anthropic error category enum.
///
/// Forward-compatible via [`ErrorType::Unknown`]: error categories the
/// Anthropic API adds after this SDK release deserialize as `Unknown`
/// rather than failing the envelope decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorType {
    /// Request failed validation (HTTP 400).
    InvalidRequestError,
    /// API key missing or invalid (HTTP 401).
    AuthenticationError,
    /// API key valid but not authorized for this resource (HTTP 403).
    PermissionError,
    /// Requested resource does not exist (HTTP 404).
    NotFoundError,
    /// Request payload exceeded the per-request size cap (HTTP 413).
    RequestTooLarge,
    /// Rate limit exceeded (HTTP 429).
    RateLimitError,
    /// Internal server error (HTTP 500).
    ApiError,
    /// Service temporarily overloaded (HTTP 529).
    OverloadedError,
    /// Category not recognized by this SDK release.
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::InvalidRequestError => "invalid_request_error",
            Self::AuthenticationError => "authentication_error",
            Self::PermissionError => "permission_error",
            Self::NotFoundError => "not_found_error",
            Self::RequestTooLarge => "request_too_large",
            Self::RateLimitError => "rate_limit_error",
            Self::ApiError => "api_error",
            Self::OverloadedError => "overloaded_error",
            Self::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

impl ApiError {
    /// Whether the failure is safe to retry with the same request body.
    ///
    /// Retryable categories: `RateLimitError`, `OverloadedError`, `ApiError`.
    /// All other categories indicate a client-side issue that retrying
    /// will not resolve.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self.body.kind,
            ErrorType::RateLimitError | ErrorType::OverloadedError | ErrorType::ApiError
        )
    }
}

/// Constructor-time failure returned by `ClientBuilder::build`.
///
/// Distinct from runtime [`TransportError`] / [`ApiError`] because
/// these failures are detectable before any request is sent.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BuildError {
    /// `base_url` could not be parsed as a valid URL.
    #[error("invalid base URL: {0}")]
    BaseUrl(String),
    /// Underlying transport construction failed.
    #[error("transport construction failed: {0}")]
    Transport(String),
    /// Configuration values failed validation.
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

/// Top-level SDK error wrapper.
///
/// Every fallible public SDK call returns `Result<T, Error>`. Match on
/// the variant to recover the layered failure domain:
///
/// - [`Error::Transport`] — wraps [`TransportError`].
/// - [`Error::Api`] — wraps [`ApiError`].
/// - [`Error::UndecodableApiError`] — non-2xx response with a body that
///   could not be parsed as the Anthropic error envelope.
/// - [`Error::Serde`] — JSON serialize/deserialize failure inside the SDK.
/// - [`Error::InvalidRequest`] — client-side rejection of a request the
///   SDK can detect without round-tripping to the API.
/// - [`Error::Build`] — wraps [`BuildError`] from client construction.
/// - [`Error::Stream`] — SSE protocol-level error encountered while
///   consuming a streaming response (`messages-streaming` feature).
///
/// Use [`Error::is_retryable`], [`Error::retry_after`], and
/// [`Error::request_id`] to inspect retry policy and correlation
/// metadata without matching variants by hand.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Transport-layer failure (network, TLS, timeout, body framing).
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// API-layer failure with a decoded error envelope.
    #[error(transparent)]
    Api(#[from] ApiError),

    /// Non-2xx response with a body that did not decode as the Anthropic
    /// error envelope. `detail` is the raw body text (truncated if very large).
    #[error("undecodable error response (status {status}): {detail}")]
    UndecodableApiError {
        /// HTTP status code.
        status: StatusCode,
        /// Raw response body or a transport-layer description of why it could not be read.
        detail: String,
        /// `request-id` echo header, if present.
        request_id: Option<RequestId>,
    },

    /// JSON serialize/deserialize failure inside the SDK.
    #[error("serialization error in {context}: {source}")]
    Serde {
        /// Where in the SDK the serialization failed (e.g. `"MessageRequest"`).
        context: &'static str,
        /// Underlying `serde_json` error.
        source: serde_json::Error,
    },

    /// Client-side rejection of a request the SDK can detect before
    /// reaching the network.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Client construction failed at build time.
    #[error(transparent)]
    Build(#[from] BuildError),

    /// SSE protocol-level error while consuming a streaming response.
    ///
    /// Returned when the SSE transport layer cannot parse a frame or
    /// the underlying byte stream fails. Non-retryable: the stream is
    /// already consumed and cannot be rewound.
    #[cfg(feature = "messages-streaming")]
    #[cfg_attr(docsrs, doc(cfg(feature = "messages-streaming")))]
    #[error("stream protocol error: {0}")]
    Stream(String),

    /// JSONL parse error while consuming a batch results stream.
    ///
    /// Returned when a line in the batch results body cannot be decoded as
    /// a [`crate::messages::BatchResultRow`]. Non-retryable: the stream
    /// position is already advanced.
    #[cfg(feature = "messages-batches")]
    #[cfg_attr(docsrs, doc(cfg(feature = "messages-batches")))]
    #[error("JSONL stream parser error: {0}")]
    JsonLines(String),
}

impl Error {
    /// Whether the failure is safe to retry with the same request body.
    ///
    /// Delegates to the underlying transport / API retry classification.
    /// Non-retryable: serde failures, undecodable responses, invalid
    /// requests, build errors, and stream protocol errors.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::Transport(e) => e.is_retryable(),
            Self::Api(e) => e.is_retryable(),
            Self::UndecodableApiError { .. }
            | Self::Serde { .. }
            | Self::InvalidRequest(_)
            | Self::Build(_) => false,
            #[cfg(feature = "messages-streaming")]
            Self::Stream(_) => false,
            #[cfg(feature = "messages-batches")]
            Self::JsonLines(_) => false,
        }
    }

    /// The server-supplied `Retry-After` duration if this is an [`ApiError`]
    /// with a populated `retry_after` field.
    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Api(e) => e.retry_after,
            Self::Transport(_)
            | Self::UndecodableApiError { .. }
            | Self::Serde { .. }
            | Self::InvalidRequest(_)
            | Self::Build(_) => None,
            #[cfg(feature = "messages-streaming")]
            Self::Stream(_) => None,
            #[cfg(feature = "messages-batches")]
            Self::JsonLines(_) => None,
        }
    }

    /// The server-supplied `request-id` header value if available.
    #[must_use]
    pub const fn request_id(&self) -> Option<&RequestId> {
        match self {
            Self::Api(e) => e.request_id.as_ref(),
            Self::UndecodableApiError { request_id, .. } => request_id.as_ref(),
            Self::Transport(_) | Self::Serde { .. } | Self::InvalidRequest(_) | Self::Build(_) => {
                None
            }
            #[cfg(feature = "messages-streaming")]
            Self::Stream(_) => None,
            #[cfg(feature = "messages-batches")]
            Self::JsonLines(_) => None,
        }
    }

    /// The server-supplied `anthropic-organization-id` header value
    /// if this is an [`ApiError`] with a populated `organization_id` field.
    #[must_use]
    pub const fn organization_id(&self) -> Option<&OrganizationId> {
        match self {
            Self::Api(e) => e.organization_id.as_ref(),
            Self::Transport(_)
            | Self::UndecodableApiError { .. }
            | Self::Serde { .. }
            | Self::InvalidRequest(_)
            | Self::Build(_) => None,
            #[cfg(feature = "messages-streaming")]
            Self::Stream(_) => None,
            #[cfg(feature = "messages-batches")]
            Self::JsonLines(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn error_type_serde_unknown_falls_back() {
        let j = r#"{"type":"weirdo_error","message":"oh no"}"#;
        let body: ApiErrorBody = serde_json::from_str(j).unwrap();
        assert_eq!(body.kind, ErrorType::Unknown);
        assert_eq!(body.message, "oh no");
    }

    #[test]
    fn error_type_serde_known_categories() {
        let j = r#"{"type":"rate_limit_error","message":"slow down"}"#;
        let body: ApiErrorBody = serde_json::from_str(j).unwrap();
        assert_eq!(body.kind, ErrorType::RateLimitError);
    }

    #[test]
    fn error_type_display_matches_wire() {
        assert_eq!(format!("{}", ErrorType::RateLimitError), "rate_limit_error");
        assert_eq!(format!("{}", ErrorType::Unknown), "unknown");
    }

    #[test]
    fn api_error_is_retryable_for_rate_limit() {
        let e = ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            body: ApiErrorBody {
                kind: ErrorType::RateLimitError,
                message: "slow down".into(),
            },
            request_id: None,
            organization_id: None,
            retry_after: Some(Duration::from_secs(2)),
        };
        assert!(e.is_retryable());
        let wrapped: Error = e.into();
        assert!(wrapped.is_retryable());
        assert_eq!(wrapped.retry_after(), Some(Duration::from_secs(2)));
    }

    #[test]
    fn api_error_not_retryable_for_invalid_request() {
        let e = ApiError {
            status: StatusCode::BAD_REQUEST,
            body: ApiErrorBody {
                kind: ErrorType::InvalidRequestError,
                message: "bad params".into(),
            },
            request_id: None,
            organization_id: None,
            retry_after: None,
        };
        assert!(!e.is_retryable());
    }

    #[test]
    fn error_request_id_propagates_from_api_error() {
        use crate::types::RequestId;
        let rid = RequestId::new("req_abc123").unwrap();
        let e = ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: ApiErrorBody {
                kind: ErrorType::ApiError,
                message: "boom".into(),
            },
            request_id: Some(rid),
            organization_id: None,
            retry_after: None,
        };
        let wrapped: Error = e.into();
        assert_eq!(
            wrapped.request_id().map(RequestId::as_str),
            Some("req_abc123")
        );
    }

    #[test]
    fn build_error_does_not_retry() {
        let e: Error = BuildError::BaseUrl("not a url".into()).into();
        assert!(!e.is_retryable());
        assert!(e.retry_after().is_none());
        assert!(e.request_id().is_none());
    }

    #[test]
    fn error_organization_id_propagates_from_api_error() {
        use crate::types::OrganizationId;
        let org = OrganizationId::new("org_xyz789").unwrap();
        let e = ApiError {
            status: StatusCode::OK,
            body: ApiErrorBody {
                kind: ErrorType::ApiError,
                message: "irrelevant".into(),
            },
            request_id: None,
            organization_id: Some(org),
            retry_after: None,
        };
        let wrapped: Error = e.into();
        assert_eq!(
            wrapped.organization_id().map(OrganizationId::as_str),
            Some("org_xyz789")
        );
    }
}
