//! A layer that meters token and cost usage off the message stream.
//!
//! Usage is reported only on the terminal result frame in this SDK's message
//! model, so the meter taps the stream and, for each result frame, adds its
//! input/output tokens and cost and increments a result-frame counter. The
//! running totals are shared with a cloneable handle the caller reads any time.
//! Frames pass through unchanged.

use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;

use crate::agent::capabilities::Capabilities;
use crate::agent::error::AgentError;
use crate::agent::message::Message;
use crate::agent::middleware::layer::Layer;
use crate::agent::middleware::tap::Tap;
use crate::agent::permissions::PermissionMode;
use crate::agent::runtime::Runtime;
use crate::agent::stream::MessageStream;
use crate::agent::types::{McpStatus, Prompt};
use crate::types::ModelId;

/// Aggregated usage read off the message stream by a [`TokenMeter`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageTotals {
    /// Summed input tokens across result frames that reported usage.
    pub input_tokens: u64,
    /// Summed output tokens across result frames that reported usage.
    pub output_tokens: u64,
    /// Summed prompt-cache creation tokens across result frames.
    pub cache_creation_input_tokens: u64,
    /// Summed prompt-cache read tokens across result frames.
    pub cache_read_input_tokens: u64,
    /// Summed `total_cost_usd` across result frames that reported it.
    pub total_cost_usd: f64,
    /// Count of result frames observed.
    pub result_frames: u64,
}

/// A cloneable read handle over a [`TokenMeter`]'s running totals.
#[derive(Clone)]
pub struct MeterHandle {
    totals: Arc<Mutex<UsageTotals>>,
}

impl MeterHandle {
    /// Snapshot the totals accumulated so far.
    #[must_use]
    pub fn totals(&self) -> UsageTotals {
        self.totals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Layer that meters usage off the message stream.
pub struct TokenMeter {
    totals: Arc<Mutex<UsageTotals>>,
}

impl TokenMeter {
    /// Build a meter layer and the handle that reads its totals.
    #[must_use]
    pub fn new() -> (Self, MeterHandle) {
        let totals = Arc::new(Mutex::new(UsageTotals::default()));
        (
            Self {
                totals: Arc::clone(&totals),
            },
            MeterHandle { totals },
        )
    }
}

impl<R: Runtime> Layer<R> for TokenMeter {
    type Runtime = MeterRuntime<R>;
    fn layer(self, inner: R) -> MeterRuntime<R> {
        MeterRuntime {
            inner,
            totals: self.totals,
        }
    }
}

/// Runtime wrapper that aggregates usage from each result frame.
pub struct MeterRuntime<R> {
    inner: R,
    totals: Arc<Mutex<UsageTotals>>,
}

fn accumulate(totals: &Arc<Mutex<UsageTotals>>, message: &Message) {
    if let Message::Result(result) = message {
        let mut totals = totals.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(usage) = &result.usage {
            totals.input_tokens += usage.input_tokens;
            totals.output_tokens += usage.output_tokens;
            totals.cache_creation_input_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
            totals.cache_read_input_tokens += usage.cache_read_input_tokens.unwrap_or(0);
        }
        if let Some(cost) = result.total_cost_usd {
            totals.total_cost_usd += cost;
        }
        totals.result_frames += 1;
    }
}

#[async_trait]
impl<R: Runtime> Runtime for MeterRuntime<R> {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
        let stream = self.inner.run(prompt).await?;
        let totals = Arc::clone(&self.totals);
        let tapped = Tap::new(stream, move |item| {
            if let Ok(message) = item {
                accumulate(&totals, message);
            }
        });
        Ok(Box::pin(tapped))
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

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{TokenMeter, UsageTotals};
    use crate::agent::message::{Message, ResultMessage, Usage};
    use crate::agent::middleware::layer::Layer;
    use crate::agent::runtime::Runtime;
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::types::{Prompt, SessionId};
    use futures_util::StreamExt;

    fn result_with_usage(input: u64, output: u64, cost: f64) -> Message {
        Message::Result(ResultMessage {
            result: "done".into(),
            structured_output: None,
            is_error: false,
            total_cost_usd: Some(cost),
            stop_reason: None,
            usage: Some(Usage {
                input_tokens: input,
                output_tokens: output,
                ..Default::default()
            }),
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })
    }

    fn result_with_cache(input: u64, output: u64, creation: u64, read: u64) -> Message {
        Message::Result(ResultMessage {
            result: "done".into(),
            structured_output: None,
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: Some(Usage {
                input_tokens: input,
                output_tokens: output,
                cache_creation_input_tokens: Some(creation),
                cache_read_input_tokens: Some(read),
            }),
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })
    }

    #[tokio::test]
    async fn aggregates_cache_counters_across_result_frames() {
        let (meter, handle) = TokenMeter::new();
        let runtime = meter.layer(MockRuntime::new(vec![
            vec![result_with_cache(10, 5, 100, 0)],
            vec![result_with_cache(3, 1, 0, 100)],
        ]));
        for _ in 0..2 {
            let mut stream = runtime.run(Prompt::from("hi")).await.expect("run");
            while stream.next().await.is_some() {}
        }
        let totals = handle.totals();
        assert_eq!(totals.cache_creation_input_tokens, 100);
        assert_eq!(totals.cache_read_input_tokens, 100);
    }

    #[tokio::test]
    async fn aggregates_usage_from_result_frames() {
        let (meter, handle) = TokenMeter::new();
        let runtime = meter.layer(MockRuntime::new(vec![
            vec![result_with_usage(10, 5, 0.02)],
            vec![result_with_usage(7, 3, 0.01)],
        ]));

        // Drive both turns to completion so every frame is tapped.
        for _ in 0..2 {
            let mut stream = runtime.run(Prompt::from("hi")).await.expect("run");
            while stream.next().await.is_some() {}
        }

        assert_eq!(
            handle.totals(),
            UsageTotals {
                input_tokens: 17,
                output_tokens: 8,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
                total_cost_usd: 0.03,
                result_frames: 2,
            }
        );
    }

    #[tokio::test]
    async fn forwards_frames_unchanged() {
        let (meter, _handle) = TokenMeter::new();
        let runtime = meter.layer(MockRuntime::new(vec![vec![result_with_usage(1, 1, 0.0)]]));
        let mut stream = runtime.run(Prompt::from("hi")).await.expect("run");
        let first = stream.next().await.expect("item").expect("ok");
        assert!(matches!(first, Message::Result(r) if r.result == "done"));
        assert!(stream.next().await.is_none());
    }
}
