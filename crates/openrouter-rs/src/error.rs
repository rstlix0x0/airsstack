//! Error types for the SDK.
//!
//! The error surface is layered so callers match on the failure domain
//! without parsing strings:
//!
//! - [`TransportError`] — HTTP transport-layer failures (network, TLS,
//!   timeout, body framing).
//! - [`BuildError`] — client-construction failures detectable before any
//!   request is sent.
//! - [`Error`] — top-level wrapper every fallible public call returns.

use std::time::Duration;

/// Failures originating in the HTTP transport layer.
///
/// Each variant maps to a failure category the SDK distinguishes without
/// inspecting message strings. Use [`TransportError::is_retryable`] to decide
/// whether a request can be safely re-issued with the same body.
///
/// # Examples
///
/// ```
/// use openrouter_rs::error::TransportError;
/// use std::time::Duration;
///
/// assert!(TransportError::Network("connection refused".into()).is_retryable());
/// assert!(!TransportError::Tls("bad certificate".into()).is_retryable());
/// assert!(TransportError::Timeout { elapsed: Duration::from_secs(30) }.is_retryable());
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// Network-level failure (connection refused, reset, DNS, etc.).
    #[error("network failure: {0}")]
    Network(String),

    /// TLS handshake or certificate validation failure.
    #[error("TLS error: {0}")]
    Tls(String),

    /// Request did not complete within the configured timeout.
    #[error("request timed out after {elapsed:?}")]
    Timeout {
        /// How long the request was in flight before being aborted.
        elapsed: Duration,
    },

    /// Failure consuming the response body stream after headers arrived.
    #[error("response body stream error: {0}")]
    BodyStream(String),

    /// Failure constructing the outgoing request (URL parse, header value, etc.).
    #[error("request build failed: {0}")]
    Build(String),

    /// Transport failure the SDK cannot categorize more specifically.
    ///
    /// Treated as non-retryable: without a known category the SDK
    /// cannot prove a retry is safe.
    #[error("transport error: {0}")]
    Other(String),
}

impl TransportError {
    /// Whether the failure is safe to retry with the same request body.
    ///
    /// Retryable: `Network`, `Timeout` (transient connectivity). All other
    /// variants indicate a request-shape or configuration issue retrying
    /// will not resolve.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::Timeout { .. })
    }
}

/// Constructor-time failure returned by the client builder.
///
/// Distinct from runtime [`TransportError`] / [`Error::Api`] because these
/// failures are detectable before any request is sent.
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
/// Every fallible public call returns `Result<T, Error>`. Match the variant
/// to recover the failure domain. No external transport type appears here:
/// `reqwest` failures are converted into [`TransportError`] at the transport
/// boundary, so a `reqwest` version bump is never a breaking change.
///
/// # Examples
///
/// ```
/// use openrouter_rs::error::{Error, TransportError};
/// let e: Error = TransportError::Network("connection refused".into()).into();
/// assert!(e.is_retryable());
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Transport-layer failure (network, TLS, timeout, body framing).
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// Non-2xx API response (other than 429) with a decoded error envelope.
    ///
    /// `code` is OpenRouter's machine-readable code; `metadata` is the raw
    /// provider-supplied detail object when present.
    #[error("API error {status} (code {code}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// OpenRouter machine-readable error code.
        code: i32,
        /// Human-readable message from the API.
        message: String,
        /// Provider-supplied error metadata, when present.
        metadata: Option<serde_json::Value>,
    },

    /// Input flagged by a provider's moderation system (HTTP 403).
    #[error("moderation blocked by {provider_name}: {reasons:?}")]
    Moderation {
        /// Reasons the input was flagged.
        reasons: Vec<String>,
        /// The offending input text.
        flagged_input: String,
        /// Provider that flagged the input.
        provider_name: String,
        /// Model slug the request targeted.
        model_slug: String,
    },

    /// Upstream provider returned an error OpenRouter passed through.
    #[error("provider error from {provider_name}")]
    Provider {
        /// Provider that produced the error.
        provider_name: String,
        /// Raw provider error payload.
        raw: serde_json::Value,
    },

    /// Rate limited (HTTP 429). `retry_after` carries the `Retry-After`
    /// header value when the server supplied one.
    #[error("rate limited (retry after {retry_after:?})")]
    RateLimit {
        /// Server-supplied retry delay, if any.
        retry_after: Option<Duration>,
    },

    /// Non-2xx response whose body did not decode as the error envelope.
    #[error("undecodable error response (status {status}): {detail}")]
    UndecodableApiError {
        /// HTTP status code.
        status: u16,
        /// Raw body text or a transport description of why it could not be read.
        detail: String,
    },

    /// JSON serialize/deserialize failure inside the SDK.
    #[error("serialization error in {context}: {source}")]
    Serde {
        /// Where in the SDK the (de)serialization failed.
        context: &'static str,
        /// Underlying `serde_json` error.
        #[source]
        source: serde_json::Error,
    },

    /// Client-side rejection detectable before reaching the network.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Client construction failed at build time.
    #[error(transparent)]
    Build(#[from] BuildError),
}

impl Error {
    /// Whether the failure is safe to retry with the same request body.
    ///
    /// Retryable: transport `Network`/`Timeout`, `RateLimit`, and `Api`
    /// responses with status 408 (Request Timeout), 500, 502, or 503.
    /// Everything else is a client-side or non-recoverable failure.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::Transport(e) => e.is_retryable(),
            Self::RateLimit { .. } => true,
            Self::Api { status, .. } => matches!(status, 408 | 500 | 502 | 503),
            Self::Moderation { .. }
            | Self::Provider { .. }
            | Self::UndecodableApiError { .. }
            | Self::Serde { .. }
            | Self::InvalidRequest(_)
            | Self::Build(_) => false,
        }
    }

    /// The server-supplied `Retry-After` duration, if this is a `RateLimit`.
    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimit { retry_after } => *retry_after,
            _ => None,
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
    fn transport_retryable_classification() {
        assert!(TransportError::Network(String::new()).is_retryable());
        assert!(
            TransportError::Timeout {
                elapsed: Duration::from_secs(1)
            }
            .is_retryable()
        );
        assert!(!TransportError::Tls(String::new()).is_retryable());
        assert!(!TransportError::BodyStream(String::new()).is_retryable());
        assert!(!TransportError::Build(String::new()).is_retryable());
        assert!(!TransportError::Other(String::new()).is_retryable());
    }

    #[test]
    fn transport_display_messages() {
        assert_eq!(
            format!("{}", TransportError::Network("connection refused".into())),
            "network failure: connection refused"
        );
        assert!(
            format!(
                "{}",
                TransportError::Timeout {
                    elapsed: Duration::from_millis(1500)
                }
            )
            .starts_with("request timed out after")
        );
    }

    #[test]
    fn build_error_display() {
        assert_eq!(
            format!("{}", BuildError::InvalidConfig("bad value".into())),
            "invalid config: bad value"
        );
    }

    #[test]
    fn transport_error_converts_into_error() {
        let e: Error = TransportError::Network("down".into()).into();
        assert!(e.is_retryable());
        assert!(matches!(e, Error::Transport(_)));
    }

    #[test]
    fn build_error_converts_and_is_not_retryable() {
        let e: Error = BuildError::BaseUrl("nope".into()).into();
        assert!(!e.is_retryable());
        assert!(e.retry_after().is_none());
    }

    #[test]
    fn api_5xx_is_retryable_4xx_is_not() {
        let server = Error::Api {
            status: 502,
            code: 502,
            message: "bad gateway".into(),
            metadata: None,
        };
        assert!(server.is_retryable());
        let client = Error::Api {
            status: 400,
            code: 400,
            message: "bad request".into(),
            metadata: None,
        };
        assert!(!client.is_retryable());
    }

    #[test]
    fn rate_limit_is_retryable_and_carries_retry_after() {
        let e = Error::RateLimit {
            retry_after: Some(Duration::from_secs(2)),
        };
        assert!(e.is_retryable());
        assert_eq!(e.retry_after(), Some(Duration::from_secs(2)));
    }

    #[test]
    fn serde_error_source_is_some() {
        use std::error::Error as StdError;
        let json_err = serde_json::from_str::<i32>("x").unwrap_err();
        let e = Error::Serde {
            context: "test",
            source: json_err,
        };
        assert!(
            e.source().is_some(),
            "Serde variant must expose its source via std::error::Error::source()"
        );
    }

    #[test]
    fn api_408_is_retryable_400_is_not() {
        let timeout = Error::Api {
            status: 408,
            code: 408,
            message: "request timeout".into(),
            metadata: None,
        };
        assert!(timeout.is_retryable(), "HTTP 408 must be retryable");
        let bad_request = Error::Api {
            status: 400,
            code: 400,
            message: "bad request".into(),
            metadata: None,
        };
        assert!(
            !bad_request.is_retryable(),
            "HTTP 400 must not be retryable"
        );
    }

    #[test]
    fn moderation_and_provider_are_not_retryable() {
        let m = Error::Moderation {
            reasons: vec!["policy".into()],
            flagged_input: "…".into(),
            provider_name: "openai".into(),
            model_slug: "openai/gpt-4o".into(),
        };
        assert!(!m.is_retryable());
        let p = Error::Provider {
            provider_name: "anthropic".into(),
            raw: serde_json::Value::Null,
        };
        assert!(!p.is_retryable());
    }
}
