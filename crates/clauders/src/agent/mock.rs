//! In-memory `Runtime` test double with no subprocess.
//!
//! Replays scripted message turns and records the control operations it
//! receives, so session and client logic can be exercised with no backend
//! binary present. Available to downstream crates through the `__test-mocks`
//! feature, mirroring the crate's mock HTTP transport.

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
use crate::agent::types::{McpStatus, Prompt};
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
}

/// A scripted, subprocess-free [`Runtime`] implementation for tests.
pub struct MockRuntime {
    scripts: Mutex<VecDeque<Vec<Message>>>,
    calls: Mutex<Vec<ControlCall>>,
    capabilities: Capabilities,
    mcp_status: McpStatus,
}

impl MockRuntime {
    /// Build a mock that replays one queued turn per `run` call.
    #[must_use]
    pub fn new(scripts: Vec<Vec<Message>>) -> Self {
        Self {
            scripts: Mutex::new(scripts.into()),
            calls: Mutex::new(Vec::new()),
            capabilities: Capabilities::default(),
            mcp_status: McpStatus::default(),
        }
    }

    /// Override the capabilities the mock reports.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Override the MCP status the mock returns from `mcp_status`.
    #[must_use]
    pub fn with_mcp_status(mut self, status: McpStatus) -> Self {
        self.mcp_status = status;
        self
    }

    /// The control operations recorded so far, in call order.
    #[must_use]
    pub fn calls(&self) -> Vec<ControlCall> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn record(&self, call: ControlCall) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }
}

#[async_trait]
impl Runtime for MockRuntime {
    async fn run(&self, _prompt: Prompt) -> Result<MessageStream, AgentError> {
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

    async fn interrupt(&self) -> Result<(), AgentError> {
        self.record(ControlCall::Interrupt);
        Ok(())
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

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{ControlCall, MockRuntime};
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::permissions::PermissionMode;
    use crate::agent::runtime::Runtime;
    use crate::agent::types::{Prompt, SessionId};
    use crate::types::ModelId;
    use futures_util::StreamExt;

    fn result(text: &str) -> Message {
        Message::Result(ResultMessage {
            result: text.into(),
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
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
}
