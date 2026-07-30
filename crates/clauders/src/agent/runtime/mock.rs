//! In-memory `Runtime` test double with no subprocess.
//!
//! Replays scripted message turns and records the control operations it
//! receives, so session and client logic can be exercised with no backend
//! binary present. Compiled only under `cfg(test)`, mirroring the crate's
//! mock HTTP transport in [`crate::test_support`].

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::agent::capabilities::Capabilities;
use crate::agent::error::AgentError;
use crate::agent::message::Message;
use crate::agent::permissions::PermissionMode;
use crate::agent::runtime::Runtime;
use crate::agent::stream::{MessageStream, ReceiverStream};
use crate::agent::types::{
    BackgroundTasksResult, ContextUsage, InitializeResult, InterruptReceipt, McpStatus, Prompt,
    ReadFileResult, ReloadPluginsResult, ReloadSkillsResult, RewindFilesResult,
    SetMcpPermissionModeResult, SetMcpServersResult, UsageReport,
};
use crate::types::ModelId;

/// A control operation observed by a [`MockRuntime`], for assertions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlCall {
    /// `interrupt` was invoked.
    Interrupt,
    /// `set_model` was invoked with this model.
    SetModel(ModelId),
    /// `set_permission_mode` was invoked with this mode.
    SetPermissionMode(PermissionMode),
    /// `mcp_status` was invoked.
    McpStatus,
    /// `reconnect_mcp_server` was invoked with this server name.
    ReconnectMcpServer(String),
    /// `read_file` was invoked with this path.
    ReadFile {
        /// The path passed to `read_file`.
        path: String,
    },
    /// `toggle_mcp_server` was invoked with this server name and enabled state.
    ToggleMcpServer {
        /// The server name passed to `toggle_mcp_server`.
        server_name: String,
        /// The enabled state passed to `toggle_mcp_server`.
        enabled: bool,
    },
    /// `set_mcp_servers` was invoked.
    SetMcpServers,
    /// `set_mcp_permission_mode_override` was invoked with this server name and mode.
    SetMcpPermissionModeOverride {
        /// The server name passed to `set_mcp_permission_mode_override`.
        server_name: String,
        /// The mode passed to `set_mcp_permission_mode_override`.
        mode: String,
    },
    /// `stop_task` was invoked with this task id.
    StopTask(String),
    /// `background_tasks` was invoked.
    BackgroundTasks,
    /// `rewind_files` was invoked with this user message id.
    RewindFiles {
        /// The user message id passed to `rewind_files`.
        user_message_id: String,
    },
    /// `seed_read_state` was invoked with this path.
    SeedReadState {
        /// The path passed to `seed_read_state`.
        path: String,
    },
    /// `get_context_usage` was invoked.
    GetContextUsage,
    /// `get_usage` was invoked.
    GetUsage,
    /// `reload_plugins` was invoked.
    ReloadPlugins,
    /// `reload_skills` was invoked.
    ReloadSkills,
    /// `apply_flag_settings` was invoked.
    ApplyFlagSettings,
    /// `set_max_thinking_tokens` was invoked.
    SetMaxThinkingTokens,
    /// `reinitialize` was invoked.
    Reinitialize,
}

/// A scripted, subprocess-free [`Runtime`] implementation for tests.
pub struct MockRuntime {
    scripts: Mutex<VecDeque<Vec<Message>>>,
    calls: Mutex<Vec<ControlCall>>,
    prompts: Mutex<Vec<Vec<String>>>,
    capabilities: Capabilities,
    mcp_status: McpStatus,
    read_file: ReadFileResult,
    initialize_result: InitializeResult,
}

impl MockRuntime {
    /// Build a mock that replays one queued turn per `run` call.
    #[must_use]
    pub fn new(scripts: Vec<Vec<Message>>) -> Self {
        Self {
            scripts: Mutex::new(scripts.into()),
            calls: Mutex::new(Vec::new()),
            prompts: Mutex::new(Vec::new()),
            capabilities: Capabilities::default(),
            mcp_status: McpStatus::default(),
            read_file: ReadFileResult::default(),
            initialize_result: InitializeResult::default(),
        }
    }

    /// The control operations recorded so far, in call order.
    #[must_use]
    pub fn calls(&self) -> Vec<ControlCall> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The prompt inputs recorded so far, one inner `Vec` per `run` call: a
    /// single-turn prompt records one element; a streamed prompt records its
    /// drained items in order.
    #[must_use]
    pub fn prompts(&self) -> Vec<Vec<String>> {
        self.prompts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Override the capability manifest this mock advertises.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Override the `initialize` response this mock retains.
    #[must_use]
    pub fn with_initialize_result(mut self, initialize_result: InitializeResult) -> Self {
        self.initialize_result = initialize_result;
        self
    }

    fn record(&self, call: ControlCall) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }

    async fn record_prompt(&self, prompt: Prompt) {
        use futures_util::StreamExt;

        let recorded = match prompt {
            Prompt::Single(text) => vec![text],
            Prompt::Stream(mut stream) => {
                let mut items = Vec::new();
                while let Some(text) = stream.next().await {
                    items.push(text);
                }
                items
            }
        };
        self.prompts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(recorded);
    }
}

#[async_trait]
impl Runtime for MockRuntime {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
        self.record_prompt(prompt).await;
        let turn = self
            .scripts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .unwrap_or_default();
        let capacity = turn.len().max(1);
        let (tx, rx) = mpsc::channel(capacity);
        for msg in turn {
            // Capacity covers every queued message, so this never blocks.
            let _ = tx.try_send(Ok(msg));
        }
        drop(tx);
        Ok(ReceiverStream::new(rx).boxed())
    }

    async fn interrupt(&self) -> Result<Option<InterruptReceipt>, AgentError> {
        self.record(ControlCall::Interrupt);
        Ok(None)
    }

    async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
        self.record(ControlCall::SetModel(model));
        Ok(())
    }

    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
        self.record(ControlCall::SetPermissionMode(mode));
        Ok(())
    }

    async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
        self.record(ControlCall::McpStatus);
        Ok(self.mcp_status.clone())
    }

    async fn reconnect_mcp_server(&self, server_name: &str) -> Result<(), AgentError> {
        self.record(ControlCall::ReconnectMcpServer(server_name.to_string()));
        Ok(())
    }

    async fn read_file(
        &self,
        path: &str,
        _max_bytes: Option<u64>,
        _encoding: Option<String>,
    ) -> Result<ReadFileResult, AgentError> {
        self.record(ControlCall::ReadFile {
            path: path.to_string(),
        });
        Ok(self.read_file.clone())
    }

    async fn toggle_mcp_server(&self, server_name: &str, enabled: bool) -> Result<(), AgentError> {
        self.record(ControlCall::ToggleMcpServer {
            server_name: server_name.to_string(),
            enabled,
        });
        Ok(())
    }

    async fn set_mcp_servers(
        &self,
        _servers: serde_json::Value,
    ) -> Result<SetMcpServersResult, AgentError> {
        self.record(ControlCall::SetMcpServers);
        Ok(SetMcpServersResult::default())
    }

    async fn set_mcp_permission_mode_override(
        &self,
        server_name: &str,
        mode: &str,
    ) -> Result<SetMcpPermissionModeResult, AgentError> {
        self.record(ControlCall::SetMcpPermissionModeOverride {
            server_name: server_name.to_string(),
            mode: mode.to_string(),
        });
        Ok(SetMcpPermissionModeResult::default())
    }

    async fn stop_task(&self, task_id: &str) -> Result<(), AgentError> {
        self.record(ControlCall::StopTask(task_id.to_string()));
        Ok(())
    }

    async fn background_tasks(
        &self,
        _tool_use_id: Option<String>,
    ) -> Result<BackgroundTasksResult, AgentError> {
        self.record(ControlCall::BackgroundTasks);
        Ok(BackgroundTasksResult::default())
    }

    async fn rewind_files(
        &self,
        user_message_id: &str,
        _dry_run: Option<bool>,
    ) -> Result<RewindFilesResult, AgentError> {
        self.record(ControlCall::RewindFiles {
            user_message_id: user_message_id.to_string(),
        });
        Ok(RewindFilesResult::default())
    }

    async fn seed_read_state(
        &self,
        path: &str,
        _mtime: serde_json::Value,
    ) -> Result<(), AgentError> {
        self.record(ControlCall::SeedReadState {
            path: path.to_string(),
        });
        Ok(())
    }

    async fn get_context_usage(&self) -> Result<ContextUsage, AgentError> {
        self.record(ControlCall::GetContextUsage);
        Ok(ContextUsage::default())
    }

    async fn get_usage(&self) -> Result<UsageReport, AgentError> {
        self.record(ControlCall::GetUsage);
        Ok(UsageReport::default())
    }

    async fn reload_plugins(&self) -> Result<ReloadPluginsResult, AgentError> {
        self.record(ControlCall::ReloadPlugins);
        Ok(ReloadPluginsResult::default())
    }

    async fn reload_skills(&self) -> Result<ReloadSkillsResult, AgentError> {
        self.record(ControlCall::ReloadSkills);
        Ok(ReloadSkillsResult::default())
    }

    async fn apply_flag_settings(&self, _settings: serde_json::Value) -> Result<(), AgentError> {
        self.record(ControlCall::ApplyFlagSettings);
        Ok(())
    }

    async fn set_max_thinking_tokens(
        &self,
        _max_thinking_tokens: Option<u64>,
        _thinking_display: Option<serde_json::Value>,
    ) -> Result<(), AgentError> {
        self.record(ControlCall::SetMaxThinkingTokens);
        Ok(())
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }

    fn initialize_result(&self) -> InitializeResult {
        self.initialize_result.clone()
    }

    async fn reinitialize(&self) -> Result<InitializeResult, AgentError> {
        self.record(ControlCall::Reinitialize);
        Ok(self.initialize_result.clone())
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{ControlCall, MockRuntime};
    use crate::agent::message::{Message, ResultMessage, ResultSubtype};
    use crate::agent::permissions::PermissionMode;
    use crate::agent::runtime::Runtime;
    use crate::agent::types::{Prompt, SessionId};
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
    async fn replays_scripted_turns_in_order() {
        let mock = MockRuntime::new(vec![vec![result("a")], vec![result("b")]]);
        let mut s1 = mock.run(Prompt::new("p1")).await.expect("run");
        assert!(matches!(s1.next().await, Some(Ok(Message::Result(r))) if r.result == "a"));
        assert!(s1.next().await.is_none());
        let mut s2 = mock.run(Prompt::new("p2")).await.expect("run");
        assert!(matches!(s2.next().await, Some(Ok(Message::Result(r))) if r.result == "b"));
    }

    #[tokio::test]
    async fn records_control_calls() {
        let mock = MockRuntime::new(vec![]);
        mock.interrupt().await.expect("interrupt");
        mock.set_model(ModelId::custom("m1").expect("model"))
            .await
            .expect("set_model");
        mock.set_permission_mode(PermissionMode::AcceptEdits)
            .await
            .expect("mode");
        let calls = mock.calls();
        assert!(matches!(
            calls.as_slice(),
            [
                ControlCall::Interrupt,
                ControlCall::SetModel(_),
                ControlCall::SetPermissionMode(PermissionMode::AcceptEdits)
            ]
        ));
    }

    #[tokio::test]
    async fn exhausted_script_yields_empty_stream() {
        let mock = MockRuntime::new(vec![]);
        let mut s = mock.run(Prompt::new("p")).await.expect("run");
        assert!(s.next().await.is_none());
    }

    #[tokio::test]
    async fn records_set_permission_mode_dont_ask() {
        let mock = MockRuntime::new(vec![]);
        mock.set_permission_mode(PermissionMode::DontAsk)
            .await
            .expect("set_permission_mode");
        assert_eq!(
            mock.calls().last().cloned(),
            Some(ControlCall::SetPermissionMode(PermissionMode::DontAsk))
        );
    }

    #[tokio::test]
    async fn records_single_prompt_as_one_element() {
        let mock = MockRuntime::new(vec![]);
        let _ = mock.run(Prompt::new("solo")).await.expect("run");
        assert_eq!(mock.prompts(), vec![vec!["solo".to_string()]]);
    }

    #[tokio::test]
    async fn records_streamed_prompt_items_in_order() {
        let mock = MockRuntime::new(vec![]);
        let stream =
            futures_util::stream::iter(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let _ = mock.run(Prompt::stream(stream)).await.expect("run");
        assert_eq!(
            mock.prompts(),
            vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]]
        );
    }
}
