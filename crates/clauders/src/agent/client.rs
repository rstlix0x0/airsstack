//! The stateful client over a runtime.
//!
//! `Client` owns a [`Runtime`] and exposes the session surface: send a prompt
//! and stream the turn, and issue live control operations. It is concrete and
//! generic over the runtime, defaulting to the subprocess-backed adapter.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use crate::agent::capabilities::Capabilities;
use crate::agent::error::AgentError;
use crate::agent::message::Message;
use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;
use crate::agent::runtime::Runtime;
use crate::agent::runtime::cli::CliRuntime;
use crate::agent::stream::MessageStream;
use crate::agent::types::{
    BackgroundTasksResult, ContextUsage, InitializeResult, InterruptReceipt, McpStatus, Prompt,
    ReadFileResult, ReloadPluginsResult, ReloadSkillsResult, RewindFilesResult,
    SetMcpPermissionModeResult, SetMcpServersResult, UsageReport,
};
use crate::agent::warm::WarmQuery;
use crate::types::ModelId;

/// A stateful agent session over a [`Runtime`].
pub struct Client<R: Runtime = CliRuntime> {
    runtime: R,
}

impl<R: Runtime> Client<R> {
    /// Build a client over an explicit runtime (e.g. a test double).
    pub const fn with_runtime(runtime: R) -> Self {
        Self { runtime }
    }

    /// Borrow the underlying runtime.
    pub const fn runtime(&self) -> &R {
        &self.runtime
    }

    /// Send `prompt` and stream the message frames of the turn.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the prompt cannot be delivered.
    pub async fn query(&self, prompt: impl Into<Prompt>) -> Result<MessageStream, AgentError> {
        self.runtime.run(prompt.into()).await
    }

    /// Interrupt the in-flight turn.
    ///
    /// Returns `Some` receipt when the backend reports which queued items
    /// remain after the interrupt, `None` when it reports nothing.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn interrupt(&self) -> Result<Option<InterruptReceipt>, AgentError> {
        self.runtime.interrupt().await
    }

    /// Switch the active model mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
        self.runtime.set_model(model).await
    }

    /// Switch the permission mode mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
        self.runtime.set_permission_mode(mode).await
    }

    /// Query the status of the configured MCP servers.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response
    /// cannot be decoded.
    pub async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
        self.runtime.mcp_status().await
    }

    /// Reconnect an MCP server mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn reconnect_mcp_server(&self, server_name: &str) -> Result<(), AgentError> {
        self.runtime.reconnect_mcp_server(server_name).await
    }

    /// Read a workspace file through the backend.
    ///
    /// # Examples
    /// ```no_run
    /// # use clauders::agent::{Client, types::ReadFileResult};
    /// # async fn f(c: &Client) -> Result<(), clauders::agent::AgentError> {
    /// let file: ReadFileResult = c.read_file("/etc/hosts", Some(1024), None).await?;
    /// println!("{}", file.abs_path);
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn read_file(
        &self,
        path: &str,
        max_bytes: Option<u64>,
        encoding: Option<String>,
    ) -> Result<ReadFileResult, AgentError> {
        self.runtime.read_file(path, max_bytes, encoding).await
    }

    /// Toggle an MCP server on or off mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn toggle_mcp_server(
        &self,
        server_name: &str,
        enabled: bool,
    ) -> Result<(), AgentError> {
        self.runtime.toggle_mcp_server(server_name, enabled).await
    }

    /// Replace the MCP server set mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn set_mcp_servers(
        &self,
        servers: serde_json::Value,
    ) -> Result<SetMcpServersResult, AgentError> {
        self.runtime.set_mcp_servers(servers).await
    }

    /// Override an MCP server's permission mode mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn set_mcp_permission_mode_override(
        &self,
        server_name: &str,
        mode: &str,
    ) -> Result<SetMcpPermissionModeResult, AgentError> {
        self.runtime
            .set_mcp_permission_mode_override(server_name, mode)
            .await
    }

    /// Stop a running task mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn stop_task(&self, task_id: &str) -> Result<(), AgentError> {
        self.runtime.stop_task(task_id).await
    }

    /// Move tool calls to the background.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn background_tasks(
        &self,
        tool_use_id: Option<String>,
    ) -> Result<BackgroundTasksResult, AgentError> {
        self.runtime.background_tasks(tool_use_id).await
    }

    /// Rewind workspace file state to a prior user message.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn rewind_files(
        &self,
        user_message_id: &str,
        dry_run: Option<bool>,
    ) -> Result<RewindFilesResult, AgentError> {
        self.runtime.rewind_files(user_message_id, dry_run).await
    }

    /// Seed a file's read state, so a later edit is not rejected as unread.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn seed_read_state(
        &self,
        path: &str,
        mtime: serde_json::Value,
    ) -> Result<(), AgentError> {
        self.runtime.seed_read_state(path, mtime).await
    }

    /// Report a breakdown of current context-window usage.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn get_context_usage(&self) -> Result<ContextUsage, AgentError> {
        self.runtime.get_context_usage().await
    }

    /// Report session cost/token usage.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn get_usage(&self) -> Result<UsageReport, AgentError> {
        self.runtime.get_usage().await
    }

    /// Reload plugins mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn reload_plugins(&self) -> Result<ReloadPluginsResult, AgentError> {
        self.runtime.reload_plugins().await
    }

    /// Reload skills mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails or its response cannot be decoded.
    pub async fn reload_skills(&self) -> Result<ReloadSkillsResult, AgentError> {
        self.runtime.reload_skills().await
    }

    /// Apply flag settings mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn apply_flag_settings(&self, settings: serde_json::Value) -> Result<(), AgentError> {
        self.runtime.apply_flag_settings(settings).await
    }

    /// Set the max thinking-token budget mid-session.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn set_max_thinking_tokens(
        &self,
        max_thinking_tokens: Option<u64>,
        thinking_display: Option<serde_json::Value>,
    ) -> Result<(), AgentError> {
        self.runtime
            .set_max_thinking_tokens(max_thinking_tokens, thinking_display)
            .await
    }

    /// The capabilities the backend advertised.
    pub fn capabilities(&self) -> Capabilities {
        self.runtime.capabilities()
    }

    /// The retained `initialize` response.
    pub fn initialize_result(&self) -> InitializeResult {
        self.runtime.initialize_result()
    }

    /// Available slash commands, from the retained `initialize` response.
    pub fn supported_commands(&self) -> Vec<serde_json::Value> {
        self.runtime.initialize_result().commands
    }

    /// Available models, from the retained `initialize` response.
    pub fn supported_models(&self) -> Vec<serde_json::Value> {
        self.runtime.initialize_result().models
    }

    /// Available agents, from the retained `initialize` response.
    pub fn supported_agents(&self) -> Vec<serde_json::Value> {
        self.runtime.initialize_result().agents
    }

    /// Account info, from the retained `initialize` response.
    pub fn account_info(&self) -> serde_json::Value {
        self.runtime.initialize_result().account
    }

    /// Re-run the `initialize` handshake over the live control channel.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn reinitialize(&self) -> Result<InitializeResult, AgentError> {
        self.runtime.reinitialize().await
    }
}

impl Client<CliRuntime> {
    /// Start building a client over the subprocess-backed runtime.
    #[must_use]
    pub fn builder() -> AgentClientBuilder {
        AgentClientBuilder::default()
    }

    /// Connect a client by spawning and handshaking with the backend.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the runtime cannot connect (discovery,
    /// spawn, version, or handshake failure).
    pub async fn connect(options: Options) -> Result<Self, AgentError> {
        Ok(Self::with_runtime(CliRuntime::connect(options).await?))
    }

    /// Pre-warm a session: spawn, handshake, and return a single-shot query handle.
    ///
    /// `connect` already spawns and completes the initialize round-trip eagerly
    /// (bounded by [`Options::control_request_timeout`]), so warm start adds only
    /// the single-shot handle. The SDK's `initializeTimeoutMs` maps to that same
    /// timeout; a distinct warm-start timeout is intentionally not added.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the runtime cannot connect.
    pub async fn startup(options: Options) -> Result<WarmQuery<CliRuntime>, AgentError> {
        Ok(WarmQuery::over(Self::connect(options).await?))
    }
}

/// Builder for a [`Client`] over the subprocess-backed runtime.
#[derive(Default)]
pub struct AgentClientBuilder {
    options: Options,
}

impl AgentClientBuilder {
    /// Set the session options.
    #[must_use]
    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// Connect using the accumulated options.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the runtime cannot connect.
    pub async fn connect(self) -> Result<Client<CliRuntime>, AgentError> {
        Client::connect(self.options).await
    }
}

/// Send one prompt to a fresh session and stream the turn.
///
/// Sugar over [`Client::connect`] + [`Client::query`]: the returned stream
/// owns the client, so the session stays alive for the lifetime of the stream
/// and is torn down when the stream is dropped.
///
/// # Errors
/// Returns an [`AgentError`] if the session cannot connect or the prompt
/// cannot be delivered.
pub async fn query(
    prompt: impl Into<Prompt>,
    options: Options,
) -> Result<MessageStream, AgentError> {
    let client = Client::connect(options).await?;
    let inner = client.query(prompt).await?;
    Ok(Box::pin(OwningStream {
        _client: client,
        inner,
    }))
}

/// A stream that keeps its owning client alive while it yields.
struct OwningStream {
    _client: Client<CliRuntime>,
    inner: MessageStream,
}

impl Stream for OwningStream {
    type Item = Result<Message, AgentError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // `Client<CliRuntime>` and the boxed inner stream are both `Unpin`.
        let this = self.get_mut();
        this.inner.as_mut().poll_next(cx)
    }
}

#[cfg(test)]
mod builder_tests {
    use super::Client;

    // Compile-time proof that `query` is a free function with the expected signature.
    // Never called; the body is dead code but the types are checked at compile time.
    async fn _assert_query_sig() {
        let _ = super::query(String::new(), crate::agent::options::Options::default()).await;
    }

    #[test]
    fn builder_defaults_to_options_default() {
        // Compiles only if AgentClientBuilder exists and Client::builder() is available.
        let _builder = Client::builder();
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::Client;
    use crate::agent::message::{Message, ResultMessage, ResultSubtype};
    use crate::agent::permissions::PermissionMode;
    use crate::agent::runtime::mock::{ControlCall, MockRuntime};
    use crate::agent::types::{ReadFileResult, SessionId};
    use crate::types::ModelId;
    use futures_util::StreamExt;

    fn result(text: &str) -> Message {
        Message::Result(ResultMessage {
            subtype: ResultSubtype::Success,
            errors: Vec::new(),
            result: text.into(),
            structured_output: None,
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
            model_usage: std::collections::HashMap::new(),
            permission_denials: Vec::new(),
            duration_ms: None,
            duration_api_ms: None,
            ttft_ms: None,
            terminal_reason: None,
            uuid: None,
            extra: serde_json::Value::Null,
        })
    }

    #[tokio::test]
    async fn query_streams_the_scripted_turn() {
        let client = Client::with_runtime(MockRuntime::new(vec![vec![result("hello")]]));
        let mut stream = client.query("hi").await.expect("query");
        let first = stream.next().await.expect("one item").expect("ok");
        assert!(matches!(first, Message::Result(r) if r.result == "hello"));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn control_methods_delegate_to_runtime() {
        let client = Client::with_runtime(MockRuntime::new(vec![]));
        client.interrupt().await.expect("interrupt");
        client
            .set_model(ModelId::custom("m").expect("model"))
            .await
            .expect("set_model");
        client
            .set_permission_mode(PermissionMode::Plan)
            .await
            .expect("mode");
        client.mcp_status().await.expect("mcp_status");
        let calls = client.runtime().calls();
        assert_eq!(calls.len(), 4);
        assert!(matches!(calls[0], ControlCall::Interrupt));
        assert!(matches!(calls[3], ControlCall::McpStatus));
    }

    #[tokio::test]
    async fn reconnect_and_read_file_delegate_to_runtime() {
        let client = Client::with_runtime(MockRuntime::new(vec![]));
        client.reconnect_mcp_server("srv").await.expect("reconnect");
        let file = client
            .read_file("/tmp/x", None, None)
            .await
            .expect("read_file");
        assert_eq!(
            file,
            ReadFileResult {
                contents: String::new(),
                abs_path: String::new(),
                truncated: None,
                encoding: None
            }
        ); // mock default
        let calls = client.runtime().calls();
        assert!(matches!(calls[0], ControlCall::ReconnectMcpServer(ref s) if s == "srv"));
        assert!(matches!(calls[1], ControlCall::ReadFile { .. }));
    }

    #[tokio::test]
    async fn live_mcp_ops_delegate_to_runtime() {
        let client = Client::with_runtime(MockRuntime::new(vec![]));
        client.toggle_mcp_server("s", false).await.expect("toggle");
        client
            .set_mcp_servers(serde_json::json!({"s":{}}))
            .await
            .expect("set");
        client
            .set_mcp_permission_mode_override("s", "plan")
            .await
            .expect("override");
        let c = client.runtime().calls();
        assert!(matches!(
            c[0],
            ControlCall::ToggleMcpServer { enabled: false, .. }
        ));
        assert!(matches!(c[1], ControlCall::SetMcpServers));
        assert!(matches!(
            c[2],
            ControlCall::SetMcpPermissionModeOverride { .. }
        ));
    }

    #[tokio::test]
    async fn task_turn_workspace_ops_delegate_to_runtime() {
        let client = Client::with_runtime(MockRuntime::new(vec![]));
        client.stop_task("t").await.expect("stop");
        client
            .background_tasks(Some("tu".into()))
            .await
            .expect("bg");
        client.rewind_files("m1", Some(true)).await.expect("rewind");
        client
            .seed_read_state("/p", serde_json::json!(1))
            .await
            .expect("seed");
        let c = client.runtime().calls();
        assert!(matches!(c[0], ControlCall::StopTask(ref s) if s == "t"));
        assert!(matches!(c[1], ControlCall::BackgroundTasks));
        assert!(matches!(c[2], ControlCall::RewindFiles { .. }));
        assert!(matches!(c[3], ControlCall::SeedReadState { .. }));
    }

    #[tokio::test]
    async fn initialize_accessors_read_the_retained_result() {
        use crate::agent::types::InitializeResult;

        let init = InitializeResult {
            models: vec![serde_json::json!("claude-x")],
            ..Default::default()
        };
        let client =
            Client::with_runtime(MockRuntime::new(vec![]).with_initialize_result(init.clone()));
        assert_eq!(client.initialize_result(), init);
        assert_eq!(
            client.supported_models(),
            vec![serde_json::json!("claude-x")]
        );
    }

    #[tokio::test]
    async fn reinitialize_delegates_and_refreshes() {
        let client = Client::with_runtime(MockRuntime::new(vec![]));
        client.reinitialize().await.expect("reinitialize");
        assert!(
            client
                .runtime()
                .calls()
                .iter()
                .any(|c| matches!(c, ControlCall::Reinitialize))
        );
    }

    #[tokio::test]
    async fn introspection_ops_delegate_to_runtime() {
        let client = Client::with_runtime(MockRuntime::new(vec![]));
        client.get_context_usage().await.expect("ctx");
        client.get_usage().await.expect("usage");
        client.reload_plugins().await.expect("plugins");
        client.reload_skills().await.expect("skills");
        client
            .apply_flag_settings(serde_json::json!({"x":1}))
            .await
            .expect("flags");
        client
            .set_max_thinking_tokens(Some(100), None)
            .await
            .expect("tokens");
        assert_eq!(client.runtime().calls().len(), 6);
    }
}
