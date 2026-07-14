//! The runtime port: the single trait seam of the Agent SDK core.
//!
//! A `Runtime` drives one agent session — it sends a prompt and yields a
//! message stream, issues live control operations, and exposes the
//! capabilities negotiated with the backend. Two implementors exist: the
//! subprocess-backed `CliRuntime` (default) and the `MockRuntime` test double.
//! Everything above this trait (`Client`) is concrete and generic over it.

use async_trait::async_trait;

use crate::agent::capabilities::Capabilities;
use crate::agent::error::AgentError;
use crate::agent::permissions::PermissionMode;
use crate::agent::stream::MessageStream;
use crate::agent::types::{McpStatus, Prompt};
use crate::types::ModelId;

/// Drives a single agent session behind a uniform, swappable interface.
#[async_trait]
pub trait Runtime: Send + Sync {
    /// Send `prompt` and return the stream of message frames for the turn.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the prompt cannot be delivered to the
    /// backend (e.g. the transport has closed).
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError>;

    /// Interrupt the in-flight turn.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the
    /// transport has closed.
    async fn interrupt(&self) -> Result<(), AgentError>;

    /// Switch the active model mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the
    /// transport has closed.
    async fn set_model(&self, model: ModelId) -> Result<(), AgentError>;

    /// Switch the permission mode mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the
    /// transport has closed.
    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError>;

    /// Query the status of the configured MCP servers.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport
    /// has closed, or the response cannot be decoded.
    async fn mcp_status(&self) -> Result<McpStatus, AgentError>;

    /// The capabilities negotiated with the backend at construction.
    fn capabilities(&self) -> &Capabilities;
}

#[cfg(test)]
mod tests {
    use super::Runtime;

    #[test]
    fn runtime_is_object_safe() {
        // Compiles only if `Runtime` is dyn-safe (async-trait boxes futures).
        fn _assert(_r: &dyn Runtime) {}
    }
}
