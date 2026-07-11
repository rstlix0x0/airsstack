//! The typed agent orchestrator: run many single-turn prompts concurrently.

use std::future::{Future, poll_fn};
use std::marker::PhantomData;

use crate::agent::client::Client;
use crate::agent::error::AgentError;
use crate::agent::evals::outcome::Outcome;
use crate::agent::orchestration::collect::collect_ordered;
use crate::agent::orchestration::core::engine::{Results, drive};
use crate::agent::orchestration::core::limiter::Limiter;
use crate::agent::orchestration::limit::semaphore::SemaphoreLimiter;
use crate::agent::runtime::Runtime;
use crate::agent::types::Prompt;

/// Drives a batch of single-turn prompts across many runtimes concurrently,
/// bounded by a [`Limiter`].
///
/// Each job builds a fresh runtime from `factory` (its own backend session —
/// e.g. a `claude` subprocess), sends one prompt, drains the turn into an
/// [`Outcome`], and yields `Result<Outcome, AgentError>`. Jobs are independent:
/// one failing never aborts the batch.
pub struct Pool<R, F, L = SemaphoreLimiter> {
    factory: F,
    limiter: L,
    _runtime: PhantomData<fn() -> R>,
}

impl<R, F, L> Pool<R, F, L> {
    /// A pool that builds runtimes with `factory` and admits jobs through `limiter`.
    pub fn new(factory: F, limiter: L) -> Self {
        Self {
            factory,
            limiter,
            _runtime: PhantomData,
        }
    }
}

impl<R, F, CFut, L> Pool<R, F, L>
where
    R: Runtime + Send + 'static,
    F: Fn() -> CFut + Clone + Send + 'static,
    CFut: Future<Output = Result<R, AgentError>> + Send + 'static,
    L: Limiter + Clone + Send + 'static,
{
    /// Run every prompt concurrently, yielding `(submission id, result)` pairs as
    /// each job finishes (as-completed order).
    pub fn run(&self, prompts: Vec<Prompt>) -> Results<Result<Outcome, AgentError>> {
        let (jtx, jrx) = tokio::sync::mpsc::channel(prompts.len().max(1));
        for (id, prompt) in prompts.into_iter().enumerate() {
            let factory = self.factory.clone();
            let _ = jtx.try_send((id, run_one(factory, prompt)));
        }
        drop(jtx);
        drive(self.limiter.clone(), jrx)
    }

    /// Run every prompt concurrently and return the results in submission order.
    pub async fn run_collect(&self, prompts: Vec<Prompt>) -> Vec<Result<Outcome, AgentError>>
    where
        F: Sync,
        L: Sync,
    {
        let len = prompts.len();
        let mut stream = self.run(prompts);
        collect_ordered(&mut stream, len).await
    }
}

/// Build a fresh runtime, send one prompt, and drain the turn into an [`Outcome`].
async fn run_one<R, F, CFut>(factory: F, prompt: Prompt) -> Result<Outcome, AgentError>
where
    R: Runtime + Send + 'static,
    F: Fn() -> CFut + Send + 'static,
    CFut: Future<Output = Result<R, AgentError>> + Send + 'static,
{
    let runtime = factory().await?;
    let client = Client::with_runtime(runtime);
    let mut stream = client.query(prompt).await?;
    let mut messages = Vec::new();
    while let Some(item) = poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
        messages.push(item?);
    }
    Ok(Outcome::from_messages(messages))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

    use super::Pool;
    use crate::agent::error::AgentError;
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::orchestration::limit::semaphore::SemaphoreLimiter;
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::types::{Prompt, SessionId};

    fn result(is_error: bool) -> Message {
        Message::Result(ResultMessage {
            result: String::new(),
            structured_output: None,
            is_error,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })
    }

    fn prompts(n: usize) -> Vec<Prompt> {
        (0..n).map(|i| Prompt::from(format!("p{i}"))).collect()
    }

    #[tokio::test]
    async fn run_collect_preserves_submission_order() {
        // Every job builds a fresh clean-result mock; all succeed.
        let pool = Pool::new(
            || async { Ok::<_, AgentError>(MockRuntime::new(vec![vec![result(false)]])) },
            SemaphoreLimiter::new(2),
        );
        let out = pool.run_collect(prompts(5)).await;
        assert_eq!(out.len(), 5);
        for (i, item) in out.iter().enumerate() {
            let outcome = item.as_ref().expect("job ok");
            assert!(!outcome.is_error(), "job {i} clean");
        }
    }

    #[tokio::test]
    async fn a_factory_failure_isolates_to_one_job() {
        // The factory fails on the third construction only; other jobs still run.
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_in = calls.clone();
        let pool = Pool::new(
            move || {
                let n = calls_in.fetch_add(1, SeqCst);
                async move {
                    if n == 2 {
                        Err(AgentError::Decode("boom".into()))
                    } else {
                        Ok(MockRuntime::new(vec![vec![result(false)]]))
                    }
                }
            },
            SemaphoreLimiter::new(1), // serialize so `n` maps to submission index
        );
        let out = pool.run_collect(prompts(4)).await;
        assert_eq!(out.len(), 4);
        assert!(out[0].is_ok());
        assert!(out[1].is_ok());
        assert!(out[2].is_err(), "third job carries the factory failure");
        assert!(out[3].is_ok(), "later jobs are unaffected");
    }
}
