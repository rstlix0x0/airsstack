//! A layer that emits `tracing` diagnostics around runtime operations.
//!
//! Read-only: it logs a turn boundary, each control operation, and each
//! streamed frame, forwarding every call and frame unchanged. It never
//! introduces, masks, or reorders anything.

use async_trait::async_trait;

use crate::agent::capabilities::Capabilities;
use crate::agent::error::AgentError;
use crate::agent::middleware::layer::Layer;
use crate::agent::middleware::tap::Tap;
use crate::agent::permissions::PermissionMode;
use crate::agent::runtime::Runtime;
use crate::agent::stream::MessageStream;
use crate::agent::types::{McpStatus, Prompt};
use crate::types::ModelId;

/// Layer that wraps a runtime with `tracing` observation.
#[derive(Clone, Copy, Debug, Default)]
pub struct Trace;

impl Trace {
    /// Build a tracing layer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<R: Runtime> Layer<R> for Trace {
    type Runtime = TraceRuntime<R>;
    fn layer(self, inner: R) -> TraceRuntime<R> {
        TraceRuntime { inner }
    }
}

/// Runtime wrapper that emits `tracing` diagnostics around each operation.
pub struct TraceRuntime<R> {
    inner: R,
}

impl<R> TraceRuntime<R> {
    /// Borrow the wrapped runtime.
    pub const fn inner(&self) -> &R {
        &self.inner
    }
}

#[async_trait]
impl<R: Runtime> Runtime for TraceRuntime<R> {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
        tracing::debug!("agent turn starting");
        let stream = self.inner.run(prompt).await?;
        let tapped = Tap::new(stream, |item| match item {
            Ok(_) => tracing::trace!("agent frame"),
            Err(err) => tracing::debug!(error = %err, "agent stream error frame"),
        });
        Ok(Box::pin(tapped))
    }

    async fn interrupt(&self) -> Result<(), AgentError> {
        tracing::debug!("agent interrupt");
        self.inner.interrupt().await
    }

    async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
        tracing::debug!("agent set_model");
        self.inner.set_model(model).await
    }

    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
        tracing::debug!("agent set_permission_mode");
        self.inner.set_permission_mode(mode).await
    }

    async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
        tracing::debug!("agent mcp_status");
        self.inner.mcp_status().await
    }

    fn capabilities(&self) -> &Capabilities {
        self.inner.capabilities()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::Trace;
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::middleware::layer::{Layer, Stack};
    use crate::agent::runtime::Runtime;
    use crate::agent::runtime::mock::{ControlCall, MockRuntime};
    use crate::agent::types::{Prompt, SessionId};
    use futures_util::StreamExt;

    fn turn(text: &str) -> Vec<Message> {
        vec![Message::Result(ResultMessage {
            result: text.into(),
            structured_output: None,
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })]
    }

    #[tokio::test]
    async fn forwards_frames_unchanged() {
        let runtime = Stack::new(MockRuntime::new(vec![turn("hello")]))
            .layer(Trace::new())
            .build();
        let mut stream = runtime.run(Prompt::from("hi")).await.expect("run");
        let first = stream.next().await.expect("one item").expect("ok");
        assert!(matches!(first, Message::Result(r) if r.result == "hello"));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn forwards_control_ops_to_inner() {
        let runtime = Trace::new().layer(MockRuntime::new(vec![]));
        runtime.interrupt().await.expect("interrupt");
        runtime.mcp_status().await.expect("mcp_status");
        let calls = runtime.inner().calls();
        assert_eq!(calls, vec![ControlCall::Interrupt, ControlCall::McpStatus]);
    }
}
