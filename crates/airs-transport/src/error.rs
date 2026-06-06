//! Transport-layer error type returned by the HTTP specialization of
//! [`crate::Transport`].
//!
//! Generic over provider: wire-level failure categories (network, TLS,
//! timeout, body framing, request build) with no API-specific meaning.

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
/// use airs_transport::TransportError;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_categories() {
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
}
