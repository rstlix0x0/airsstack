//! The user-supplied [`ElicitationPolicy`] consulted per elicitation.

use async_trait::async_trait;

use crate::agent::cancel::CancelSignal;
use crate::agent::elicitation::{ElicitationRequest, ElicitationResponse};
use crate::agent::error::AgentError;

/// A user-supplied policy consulted on each elicitation request.
///
/// Registered via [`crate::agent::Options`] and consulted by the runtime's
/// background reader; the returned [`ElicitationResponse`] is sent back to
/// the binary as the correlated control response. With no policy
/// registered the runtime declines, matching the official SDK's fallback.
#[async_trait]
pub trait ElicitationPolicy: Send + Sync {
    /// Resolve `request` to an outcome.
    ///
    /// `cancel` observes whether the binary withdrew the request while this
    /// call is in flight; a policy may check it and bail out early, or ignore
    /// it and run to completion.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the policy cannot resolve the request;
    /// the runtime surfaces it to the binary as an error control response.
    async fn elicit(
        &self,
        request: ElicitationRequest,
        cancel: CancelSignal,
    ) -> Result<ElicitationResponse, AgentError>;
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::sync::Arc;

    use super::ElicitationPolicy;
    use crate::agent::elicitation::{ElicitationMode, ElicitationRequest, ElicitationResponse};
    use crate::agent::error::AgentError;

    struct AcceptPolicy;

    #[async_trait::async_trait]
    impl ElicitationPolicy for AcceptPolicy {
        async fn elicit(
            &self,
            _request: ElicitationRequest,
            _cancel: crate::agent::cancel::CancelSignal,
        ) -> Result<ElicitationResponse, AgentError> {
            Ok(ElicitationResponse::Accept(
                serde_json::json!({ "ok": true }),
            ))
        }
    }

    fn request() -> ElicitationRequest {
        ElicitationRequest {
            elicitation_id: Some("elic_1".to_string()),
            message: "Pick".to_string(),
            mode: Some(ElicitationMode::Form),
            requested_schema: None,
            url: None,
            mcp_server_name: "srv".to_string(),
            title: None,
            display_name: None,
            description: None,
        }
    }

    #[tokio::test]
    async fn policy_is_object_safe_and_returns_its_outcome() {
        let policy: Arc<dyn ElicitationPolicy> = Arc::new(AcceptPolicy);
        let outcome = policy
            .elicit(request(), crate::agent::cancel::CancelSignal::new())
            .await
            .expect("outcome");
        assert_eq!(
            outcome,
            ElicitationResponse::Accept(serde_json::json!({ "ok": true }))
        );
    }
}
