//! Error types for the SDK.
//!
//! The error surface is layered so callers can match on the failure
//! domain without parsing strings:
//!
//! - [`TransportError`] — failures in the HTTP transport layer (network,
//!   TLS, timeout, body framing).

use std::time::Duration;

/// Failures originating in the HTTP transport layer.
///
/// Each variant maps to a category of failure that the SDK can
/// distinguish without inspecting error message strings. Use
/// [`TransportError::is_retryable`] to decide whether a request can be
/// safely retried with the same body.
///
/// # Examples
///
/// ```
/// use clauders::error::TransportError;
/// use std::time::Duration;
///
/// let e = TransportError::Network("connection refused".into());
/// assert!(e.is_retryable());
///
/// let e = TransportError::Tls("certificate verification failed".into());
/// assert!(!e.is_retryable());
///
/// let e = TransportError::Timeout { elapsed: Duration::from_secs(30) };
/// assert!(e.is_retryable());
/// ```
#[derive(Debug, thiserror::Error)]
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

    /// Catch-all for transport-layer failures the SDK cannot categorize
    /// more specifically.
    #[error("transport error: {0}")]
    Other(String),
}

impl TransportError {
    /// Whether the failure is safe to retry with the same request body.
    ///
    /// Transient failures (`Network`, `Timeout`, `Other`) are retryable.
    /// Failures that indicate misconfiguration or a malformed request
    /// (`Tls`, `BodyStream`, `Build`) are not — retrying without
    /// changing the request will produce the same error.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::Timeout { .. } | Self::Other(_)
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn retryable_classification() {
        assert!(TransportError::Network(String::new()).is_retryable());
        assert!(
            TransportError::Timeout {
                elapsed: Duration::from_secs(1)
            }
            .is_retryable()
        );
        assert!(TransportError::Other(String::new()).is_retryable());

        assert!(!TransportError::Tls(String::new()).is_retryable());
        assert!(!TransportError::BodyStream(String::new()).is_retryable());
        assert!(!TransportError::Build(String::new()).is_retryable());
    }

    #[test]
    fn display_messages() {
        let e = TransportError::Network("connection refused".into());
        assert_eq!(format!("{e}"), "network failure: connection refused");

        let e = TransportError::Timeout {
            elapsed: Duration::from_millis(1500),
        };
        // Format uses `{:?}` on Duration; just verify the prefix.
        assert!(format!("{e}").starts_with("request timed out after"));

        let e = TransportError::Build("invalid header value".into());
        assert_eq!(format!("{e}"), "request build failed: invalid header value");
    }
}
