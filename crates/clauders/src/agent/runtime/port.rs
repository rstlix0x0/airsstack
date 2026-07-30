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
use crate::agent::types::{
    BackgroundTasksResult, ContextUsage, InitializeResult, InterruptReceipt, McpStatus, Prompt,
    ReadFileResult, ReloadPluginsResult, ReloadSkillsResult, RewindFilesResult,
    SetMcpPermissionModeResult, SetMcpServersResult, UsageReport,
};
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
    /// Returns `Some` receipt when the backend reports which queued items
    /// remain after the interrupt, `None` when it reports nothing.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the
    /// transport has closed.
    async fn interrupt(&self) -> Result<Option<InterruptReceipt>, AgentError>;

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

    /// Reconnect an MCP server mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the transport closed.
    async fn reconnect_mcp_server(&self, server_name: &str) -> Result<(), AgentError>;

    /// Read a workspace file through the backend.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn read_file(
        &self,
        path: &str,
        max_bytes: Option<u64>,
        encoding: Option<String>,
    ) -> Result<ReadFileResult, AgentError>;

    /// Toggle an MCP server on or off mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the transport closed.
    async fn toggle_mcp_server(&self, server_name: &str, enabled: bool) -> Result<(), AgentError>;

    /// Replace the MCP server set mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn set_mcp_servers(
        &self,
        servers: serde_json::Value,
    ) -> Result<SetMcpServersResult, AgentError>;

    /// Override an MCP server's permission mode mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn set_mcp_permission_mode_override(
        &self,
        server_name: &str,
        mode: &str,
    ) -> Result<SetMcpPermissionModeResult, AgentError>;

    /// Stop a running task mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the transport closed.
    async fn stop_task(&self, task_id: &str) -> Result<(), AgentError>;

    /// Move tool calls to the background.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn background_tasks(
        &self,
        tool_use_id: Option<String>,
    ) -> Result<BackgroundTasksResult, AgentError>;

    /// Rewind workspace file state to a prior user message.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn rewind_files(
        &self,
        user_message_id: &str,
        dry_run: Option<bool>,
    ) -> Result<RewindFilesResult, AgentError>;

    /// Seed a file's read state, so a later edit is not rejected as unread.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the transport closed.
    async fn seed_read_state(&self, path: &str, mtime: serde_json::Value)
    -> Result<(), AgentError>;

    /// Report a breakdown of current context-window usage.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn get_context_usage(&self) -> Result<ContextUsage, AgentError>;

    /// Report session cost/token usage.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn get_usage(&self) -> Result<UsageReport, AgentError>;

    /// Reload plugins mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn reload_plugins(&self) -> Result<ReloadPluginsResult, AgentError>;

    /// Reload skills mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport closed,
    /// or the response cannot be decoded.
    async fn reload_skills(&self) -> Result<ReloadSkillsResult, AgentError>;

    /// Apply flag settings mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the transport closed.
    async fn apply_flag_settings(&self, settings: serde_json::Value) -> Result<(), AgentError>;

    /// Set the max thinking-token budget mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or the transport closed.
    async fn set_max_thinking_tokens(
        &self,
        max_thinking_tokens: Option<u64>,
        thinking_display: Option<serde_json::Value>,
    ) -> Result<(), AgentError>;

    /// The capabilities the backend advertised.
    ///
    /// Empty until the binary's `system`/`init` frame has been read, so a
    /// caller that checks before the first turn sees no flags.
    fn capabilities(&self) -> Capabilities;

    /// The retained `initialize` response.
    fn initialize_result(&self) -> InitializeResult;

    /// Re-run the `initialize` handshake over the live control channel.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails, the transport
    /// closed, or the response cannot be decoded.
    async fn reinitialize(&self) -> Result<InitializeResult, AgentError>;
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
