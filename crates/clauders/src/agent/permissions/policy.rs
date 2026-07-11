//! The user-supplied [`PermissionPolicy`] trait consulted per tool call.

use async_trait::async_trait;

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
    /// # Errors
    /// Returns an [`AgentError`] if the policy cannot reach a decision; the
    /// runtime surfaces it to the binary as an error control response.
    async fn can_use_tool(
        &self,
        tool: &str,
        input: &serde_json::Value,
        ctx: PermissionContext,
    ) -> Result<PermissionDecision, AgentError>;
}
