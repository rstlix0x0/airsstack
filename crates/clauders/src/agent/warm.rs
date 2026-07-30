//! Warm start: a pre-initialized session with a single-shot query.

use std::future::Future;

use crate::agent::client::Client;
use crate::agent::error::AgentError;
use crate::agent::runtime::Runtime;
use crate::agent::runtime::cli::CliRuntime;
use crate::agent::stream::MessageStream;
use crate::agent::types::{InterruptReceipt, Prompt};

/// A pre-warmed session whose `query` may be called exactly once.
pub struct WarmQuery<R: Runtime = CliRuntime> {
    client: Client<R>,
}

impl<R: Runtime> WarmQuery<R> {
    /// Wrap an already-connected client. (`Client::startup` is the public entry.)
    pub(crate) const fn over(client: Client<R>) -> Self {
        Self { client }
    }

    /// Submit the single prompt and hand back the controllable session.
    ///
    /// Consuming `self` makes the "query only once" guarantee a compile-time
    /// property — a second call does not type-check.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the prompt cannot be delivered.
    pub async fn query(self, prompt: impl Into<Prompt>) -> Result<WarmSession<R>, AgentError> {
        let stream = self.client.query(prompt).await?;
        Ok(WarmSession {
            client: self.client,
            stream,
        })
    }

    /// Tear the warmed session down without querying.
    pub fn close(self) {
        drop(self.client);
    }
}

/// The live session returned by [`WarmQuery::query`]: the message stream plus
/// the mid-session control operations.
pub struct WarmSession<R: Runtime = CliRuntime> {
    client: Client<R>,
    stream: MessageStream,
}

impl<R: Runtime> WarmSession<R> {
    /// The turn's message stream.
    pub fn stream(&mut self) -> &mut MessageStream {
        &mut self.stream
    }

    /// Interrupt the warmed turn.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the control request fails.
    pub fn interrupt(
        &self,
    ) -> impl Future<Output = Result<Option<InterruptReceipt>, AgentError>> + '_ {
        self.client.interrupt()
    }
    // Re-expose further control ops (set_model, mcp_status, …) as needed by callers,
    // each a one-line delegate to self.client. Add only those with a concrete caller (YAGNI).
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect")]
    use crate::agent::client::Client;
    use crate::agent::message::{Message, ResultMessage, ResultSubtype};
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::types::SessionId;
    use futures_util::StreamExt;

    fn result(t: &str) -> Message {
        Message::Result(ResultMessage {
            subtype: ResultSubtype::Success,
            errors: vec![],
            result: t.into(),
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
    async fn warm_query_streams_once_and_keeps_control() {
        let warm =
            super::WarmQuery::over(Client::with_runtime(MockRuntime::new(vec![vec![result(
                "hi",
            )]])));
        let mut session = warm.query("go").await.expect("query");
        // control survives the warmed turn:
        session.interrupt().await.expect("interrupt");
        let first = session.stream().next().await.expect("item").expect("ok");
        assert!(matches!(first, Message::Result(r) if r.result == "hi"));
    }
}
