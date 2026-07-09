//! A layer that retries transient runtime failures with exponential backoff.
//!
//! Retries `run` only when it fails before yielding a stream, plus the control
//! operations. It never retries mid-stream: once `run` hands back a stream, a
//! per-item error inside that stream is forwarded untouched, because a
//! partially consumed stream cannot be replayed. On exhausting the budget the
//! last observed error is returned.

use std::time::Duration;

use async_trait::async_trait;
use tokio::time::sleep;

use crate::agent::capabilities::Capabilities;
use crate::agent::error::AgentError;
use crate::agent::middleware::layer::Layer;
use crate::agent::permissions::PermissionMode;
use crate::agent::process::ProcessError;
use crate::agent::runtime::Runtime;
use crate::agent::stream::MessageStream;
use crate::agent::types::{McpStatus, Prompt};
use crate::types::ModelId;

/// Whether `err` is a transient failure worth retrying.
const fn is_transient(err: &AgentError) -> bool {
    matches!(
        err,
        AgentError::TransportClosed
            | AgentError::Timeout
            | AgentError::Process(ProcessError::Timeout)
    )
}

/// Exponential backoff schedule: `base * 2^attempt`, capped at `cap`.
#[derive(Clone, Copy)]
struct Backoff {
    base: Duration,
    cap: Duration,
}

impl Backoff {
    fn delay(&self, attempt: u32) -> Duration {
        let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
        self.base.saturating_mul(factor).min(self.cap)
    }
}

/// Layer that retries transient failures with exponential backoff.
#[derive(Clone, Copy, Debug)]
pub struct Retry {
    max_retries: u32,
    base: Duration,
    cap: Duration,
}

impl Retry {
    /// Retry up to `max_retries` times with the default backoff
    /// (base 100ms, doubling, capped at 2s).
    #[must_use]
    pub const fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            base: Duration::from_millis(100),
            cap: Duration::from_secs(2),
        }
    }

    /// Override the base delay of the first retry.
    #[must_use]
    pub const fn base_delay(mut self, base: Duration) -> Self {
        self.base = base;
        self
    }

    /// Override the maximum delay any single retry waits.
    #[must_use]
    pub const fn max_delay(mut self, cap: Duration) -> Self {
        self.cap = cap;
        self
    }
}

impl<R: Runtime> Layer<R> for Retry {
    type Runtime = RetryRuntime<R>;
    fn layer(self, inner: R) -> RetryRuntime<R> {
        RetryRuntime {
            inner,
            max_retries: self.max_retries,
            backoff: Backoff {
                base: self.base,
                cap: self.cap,
            },
        }
    }
}

/// Runtime wrapper that retries transient failures.
///
/// Retries the initial `run` call and the control operations; never retries a
/// per-item error inside an already-returned stream.
pub struct RetryRuntime<R> {
    inner: R,
    max_retries: u32,
    backoff: Backoff,
}

impl<R> RetryRuntime<R> {
    /// Borrow the wrapped runtime.
    pub const fn inner(&self) -> &R {
        &self.inner
    }
}

impl<R: Runtime> RetryRuntime<R> {
    /// Run `op` under the retry policy, sleeping between transient failures.
    async fn with_retry<T, F, Fut>(&self, mut op: F) -> Result<T, AgentError>
    where
        F: FnMut() -> Fut,
        Fut: core::future::Future<Output = Result<T, AgentError>>,
    {
        let mut attempt = 0u32;
        loop {
            match op().await {
                Ok(value) => return Ok(value),
                Err(err) if is_transient(&err) && attempt < self.max_retries => {
                    sleep(self.backoff.delay(attempt)).await;
                    attempt += 1;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

#[async_trait]
impl<R: Runtime> Runtime for RetryRuntime<R> {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
        self.with_retry(|| self.inner.run(prompt.clone())).await
    }

    async fn interrupt(&self) -> Result<(), AgentError> {
        self.with_retry(|| self.inner.interrupt()).await
    }

    async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
        self.with_retry(|| self.inner.set_model(model.clone()))
            .await
    }

    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
        self.with_retry(|| self.inner.set_permission_mode(mode))
            .await
    }

    async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
        self.with_retry(|| self.inner.mcp_status()).await
    }

    fn capabilities(&self) -> &Capabilities {
        self.inner.capabilities()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{Retry, is_transient};
    use crate::agent::capabilities::Capabilities;
    use crate::agent::error::AgentError;
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::middleware::layer::Layer;
    use crate::agent::permissions::PermissionMode;
    use crate::agent::process::ProcessError;
    use crate::agent::runtime::Runtime;
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::stream::MessageStream;
    use crate::agent::types::{McpStatus, Prompt, SessionId};
    use crate::types::ModelId;
    use async_trait::async_trait;
    use futures_util::{StreamExt, stream};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    /// Whether the failure a flaky double returns is retryable.
    #[derive(Clone, Copy)]
    enum Kind {
        Transient,
        Terminal,
    }

    /// A runtime that fails its first `fails` gated calls, then delegates.
    struct FlakyRuntime {
        fails_remaining: Mutex<u32>,
        attempts: AtomicU32,
        kind: Kind,
        inner: MockRuntime,
    }

    impl FlakyRuntime {
        fn new(fails: u32, kind: Kind, inner: MockRuntime) -> Self {
            Self {
                fails_remaining: Mutex::new(fails),
                attempts: AtomicU32::new(0),
                kind,
                inner,
            }
        }

        fn attempts(&self) -> u32 {
            self.attempts.load(Ordering::SeqCst)
        }

        fn gate(&self) -> Option<AgentError> {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            let mut left = self.fails_remaining.lock().expect("lock");
            let has_failure_left = *left > 0;
            if has_failure_left {
                *left -= 1;
            }
            drop(left);
            has_failure_left.then(|| match self.kind {
                Kind::Transient => AgentError::TransportClosed,
                Kind::Terminal => AgentError::Protocol {
                    detail: "terminal".into(),
                },
            })
        }
    }

    #[async_trait]
    impl Runtime for FlakyRuntime {
        async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
            match self.gate() {
                Some(err) => Err(err),
                None => self.inner.run(prompt).await,
            }
        }
        async fn interrupt(&self) -> Result<(), AgentError> {
            match self.gate() {
                Some(err) => Err(err),
                None => self.inner.interrupt().await,
            }
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

    /// A runtime whose `run` yields a stream containing a single error item.
    struct MidStreamErrRuntime {
        caps: Capabilities,
    }

    #[async_trait]
    impl Runtime for MidStreamErrRuntime {
        async fn run(&self, _prompt: Prompt) -> Result<MessageStream, AgentError> {
            Ok(Box::pin(stream::once(async {
                Err(AgentError::TransportClosed)
            })))
        }
        async fn interrupt(&self) -> Result<(), AgentError> {
            Ok(())
        }
        async fn set_model(&self, _model: ModelId) -> Result<(), AgentError> {
            Ok(())
        }
        async fn set_permission_mode(&self, _mode: PermissionMode) -> Result<(), AgentError> {
            Ok(())
        }
        async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
            Ok(McpStatus::default())
        }
        fn capabilities(&self) -> &Capabilities {
            &self.caps
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

    fn fast_retry(max: u32) -> Retry {
        Retry::new(max)
            .base_delay(Duration::ZERO)
            .max_delay(Duration::ZERO)
    }

    #[test]
    fn classifier_marks_transport_and_timeout_transient() {
        assert!(is_transient(&AgentError::TransportClosed));
        assert!(is_transient(&AgentError::Timeout));
        assert!(is_transient(&AgentError::Process(ProcessError::Timeout)));
        assert!(!is_transient(&AgentError::Protocol { detail: "x".into() }));
    }

    #[tokio::test]
    async fn retries_transient_then_succeeds() {
        let flaky = FlakyRuntime::new(2, Kind::Transient, MockRuntime::new(vec![turn("ok")]));
        let runtime = fast_retry(3).layer(flaky);
        let mut stream = runtime.run(Prompt::from("hi")).await.expect("run");
        let first = stream.next().await.expect("item").expect("ok");
        assert!(matches!(first, Message::Result(r) if r.result == "ok"));
        assert_eq!(runtime.inner().attempts(), 3, "2 failures + 1 success");
    }

    #[tokio::test]
    async fn returns_last_error_when_budget_exhausted() {
        let flaky = FlakyRuntime::new(5, Kind::Transient, MockRuntime::new(vec![turn("ok")]));
        let runtime = fast_retry(2).layer(flaky);
        let err = runtime
            .run(Prompt::from("hi"))
            .await
            .err()
            .expect("exhausted");
        assert!(matches!(err, AgentError::TransportClosed));
        assert_eq!(runtime.inner().attempts(), 3, "1 initial + 2 retries");
    }

    #[tokio::test]
    async fn does_not_retry_terminal_error() {
        let flaky = FlakyRuntime::new(1, Kind::Terminal, MockRuntime::new(vec![turn("ok")]));
        let runtime = fast_retry(3).layer(flaky);
        let err = runtime
            .run(Prompt::from("hi"))
            .await
            .err()
            .expect("terminal");
        assert!(matches!(err, AgentError::Protocol { .. }));
        assert_eq!(
            runtime.inner().attempts(),
            1,
            "terminal error is not retried"
        );
    }

    #[tokio::test]
    async fn does_not_retry_mid_stream_error() {
        let runtime = fast_retry(3).layer(MidStreamErrRuntime {
            caps: Capabilities::default(),
        });
        let mut stream = runtime.run(Prompt::from("hi")).await.expect("run ok");
        let item = stream.next().await.expect("one item");
        assert!(
            matches!(item, Err(AgentError::TransportClosed)),
            "mid-stream error forwarded, not retried"
        );
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn retries_transient_control_op() {
        let flaky = FlakyRuntime::new(1, Kind::Transient, MockRuntime::new(vec![]));
        let runtime = fast_retry(3).layer(flaky);
        runtime
            .interrupt()
            .await
            .expect("interrupt eventually succeeds");
        assert_eq!(runtime.inner().attempts(), 2, "1 failure + 1 success");
    }
}
