//! The middleware composition seam: the `Layer` trait and the `Stack` builder.
//!
//! A `Layer` is a factory that wraps a runtime, producing a new runtime. A
//! `Stack` folds layers over a base runtime at the type level: the first layer
//! added is the innermost wrap (closest to the base), and each later layer
//! wraps outward, so the last layer added is the outermost — the first to
//! observe a call. The composed runtime is a concrete type handed to a client.

use crate::agent::runtime::Runtime;

/// Wraps a runtime with an additional behavior, producing a new runtime.
pub trait Layer<R: Runtime> {
    /// The runtime this layer produces.
    type Runtime: Runtime;
    /// Wrap `inner`, returning the decorated runtime.
    fn layer(self, inner: R) -> Self::Runtime;
}

/// Type-level builder that folds layers over a base runtime.
pub struct Stack<R: Runtime> {
    runtime: R,
}

impl<R: Runtime> Stack<R> {
    /// Start a stack from a base runtime.
    pub const fn new(runtime: R) -> Self {
        Self { runtime }
    }

    /// Wrap the current runtime with `layer`, growing the stack outward.
    ///
    /// The layer passed here becomes the new outermost wrap.
    pub fn layer<L: Layer<R>>(self, layer: L) -> Stack<L::Runtime> {
        Stack {
            runtime: layer.layer(self.runtime),
        }
    }

    /// Consume the stack and yield the fully composed runtime.
    pub fn build(self) -> R {
        self.runtime
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{Layer, Stack};
    use crate::agent::capabilities::Capabilities;
    use crate::agent::error::AgentError;
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::permissions::PermissionMode;
    use crate::agent::runtime::Runtime;
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::stream::MessageStream;
    use crate::agent::types::{McpStatus, Prompt, SessionId};
    use crate::types::ModelId;
    use async_trait::async_trait;
    use futures_util::StreamExt;

    /// A layer that wraps a runtime in a transparent passthrough.
    struct Identity;

    impl<R: Runtime> Layer<R> for Identity {
        type Runtime = IdentityRuntime<R>;
        fn layer(self, inner: R) -> IdentityRuntime<R> {
            IdentityRuntime { inner }
        }
    }

    struct IdentityRuntime<R> {
        inner: R,
    }

    #[async_trait]
    impl<R: Runtime> Runtime for IdentityRuntime<R> {
        async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
            self.inner.run(prompt).await
        }
        async fn interrupt(&self) -> Result<(), AgentError> {
            self.inner.interrupt().await
        }
        async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
            self.inner.set_model(model).await
        }
        async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
            self.inner.set_permission_mode(mode).await
        }
        async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
            self.inner.mcp_status().await
        }
        fn capabilities(&self) -> &Capabilities {
            self.inner.capabilities()
        }
    }

    fn turn(text: &str) -> Vec<Message> {
        vec![Message::Result(ResultMessage {
            result: text.into(),
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })]
    }

    #[test]
    fn build_returns_the_base_runtime() {
        let base = MockRuntime::new(vec![]);
        let built = Stack::new(base).build();
        // Compiles + typechecks: the base runtime is returned unchanged.
        let _: &MockRuntime = &built;
    }

    #[tokio::test]
    async fn layered_stack_delegates_a_turn() {
        let base = MockRuntime::new(vec![turn("hello")]);
        let runtime = Stack::new(base).layer(Identity).layer(Identity).build();
        let mut stream = runtime.run(Prompt::from("hi")).await.expect("run");
        let first = stream.next().await.expect("one item").expect("ok");
        assert!(matches!(first, Message::Result(r) if r.result == "hello"));
    }
}
