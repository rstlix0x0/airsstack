//! The user-supplied [`PermissionPolicy`] trait consulted per tool call.

use async_trait::async_trait;

use crate::agent::cancel::CancelSignal;
use crate::agent::error::AgentError;
use crate::agent::permissions::{PermissionContext, PermissionDecision};

/// A user-supplied policy consulted on each `can_use_tool` request.
///
/// Registered via [`crate::agent::Options`] and consulted by the runtime's
/// background reader; the returned [`PermissionDecision`] is sent back to the
/// binary as the correlated control response.
#[async_trait]
pub trait PermissionPolicy: Send + Sync {
    /// Decide whether `tool` may run with `input`.
    ///
    /// `cancel` observes whether the binary withdrew the request while this
    /// call is in flight; a policy may check it and bail out early, or ignore
    /// it and run to completion.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the policy cannot reach a decision; the
    /// runtime surfaces it to the binary as an error control response.
    async fn can_use_tool(
        &self,
        tool: &str,
        input: &serde_json::Value,
        ctx: PermissionContext,
        cancel: CancelSignal,
    ) -> Result<PermissionDecision, AgentError>;
}

#[cfg(test)]
mod tests {
    use super::PermissionPolicy;
    use crate::agent::cancel::CancelSignal;
    use crate::agent::error::AgentError;
    use crate::agent::permissions::{PermissionContext, PermissionDecision};

    #[tokio::test]
    async fn a_policy_can_observe_its_cancellation_signal() {
        struct ObservingPolicy;

        #[async_trait::async_trait]
        impl PermissionPolicy for ObservingPolicy {
            async fn can_use_tool(
                &self,
                _tool: &str,
                _input: &serde_json::Value,
                _ctx: PermissionContext,
                cancel: CancelSignal,
            ) -> Result<PermissionDecision, AgentError> {
                if cancel.is_cancelled() {
                    return Err(AgentError::Protocol {
                        detail: "cancelled".to_string(),
                    });
                }
                Ok(PermissionDecision::allow())
            }
        }

        let cancel = CancelSignal::new();
        cancel.cancel();
        let result = ObservingPolicy
            .can_use_tool(
                "Bash",
                &serde_json::json!({}),
                PermissionContext::default(),
                cancel,
            )
            .await;
        assert!(
            result.is_err(),
            "an observing policy must be able to bail out"
        );
    }
}
