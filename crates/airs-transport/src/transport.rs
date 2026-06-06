//! The generic [`Transport`] contract: send one request, get one response.
//!
//! Names no HTTP concept. The HTTP specialization is [`crate::HttpTransport`];
//! a non-HTTP transport implements `Transport` with its own associated types.
//!
//! Pure trait definition — no inline tests per the unit-test-mandate
//! exemption #3 (the trait body has no executable logic).

/// Send-one-request transport boundary.
///
/// Implementations carry no shared interpretation of the request or response
/// beyond moving one to produce the other. The HTTP specialization fixes the
/// associated types to the `http` crate types; see [`crate::HttpTransport`].
#[async_trait::async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Request the transport accepts.
    type Request: Send;
    /// Response the transport produces on success.
    type Response: Send;
    /// Error the transport produces on failure.
    type Error: Send;

    /// Send a request and return the response.
    ///
    /// # Errors
    /// Returns [`Transport::Error`] when the transport fails to produce a
    /// response. For the HTTP specialization, protocol-level non-success
    /// results (HTTP 4xx/5xx) are NOT errors at this layer.
    async fn send(&self, req: Self::Request) -> Result<Self::Response, Self::Error>;
}
