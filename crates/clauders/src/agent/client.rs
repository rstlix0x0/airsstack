//! The stateful client over a runtime.
//!
//! `Client` owns a [`Runtime`] and exposes the session surface: send a prompt
//! and stream the turn, and issue live control operations. It is concrete and
//! generic over the runtime, defaulting to the subprocess-backed adapter.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use crate::agent::capabilities::Capabilities;
use crate::agent::cli::CliRuntime;
use crate::agent::error::AgentError;
use crate::agent::message::Message;
use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;
use crate::agent::runtime::Runtime;
use crate::agent::stream::MessageStream;
use crate::agent::types::{McpStatus, Prompt};
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
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub async fn interrupt(&self) -> Result<(), AgentError> {
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

    /// The capabilities negotiated with the backend.
    pub fn capabilities(&self) -> &Capabilities {
        self.runtime.capabilities()
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
#[cfg(feature = "__test-mocks")]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::Client;
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::mock::{ControlCall, MockRuntime};
    use crate::agent::permissions::PermissionMode;
    use crate::agent::types::SessionId;
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
}
