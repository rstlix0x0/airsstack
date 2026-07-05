//! The invariant bounded-concurrency driver.
//!
//! [`drive`] consumes a channel of tagged lazy futures, admits each through a
//! [`Limiter`], runs at most the limiter allows at once, and yields each
//! `(id, output)` as its job finishes — as-completed order. It is agent-agnostic:
//! the job output type `O` is opaque. Dropping the returned [`Results`] stream
//! stops admission and aborts in-flight jobs.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::agent::orchestration::core::limiter::Limiter;

/// Output buffer for finished-but-unconsumed results before send backpressure applies.
const RESULT_BUFFER: usize = 64;

/// The as-completed result stream returned by [`drive`].
///
/// Yields `(submission id, job output)` pairs in the order jobs finish, not the
/// order they were submitted.
pub struct Results<O> {
    rx: mpsc::Receiver<(usize, O)>,
}

impl<O> Stream for Results<O> {
    type Item = (usize, O);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Drive `jobs` to completion under `limiter`, emitting results as-completed.
///
/// Each item pulled from `jobs` is `(id, future)`; the future is admitted through
/// `limiter` (acquire-before-spawn, so spawned-and-unfinished jobs never exceed the
/// limiter's bound), spawned, and its `(id, output)` forwarded to the returned
/// [`Results`] stream when it finishes. When `jobs` closes, the stream ends after
/// in-flight jobs finish. If the consumer drops the [`Results`] stream, admission
/// stops and in-flight jobs are aborted.
pub fn drive<L, Fut, O>(limiter: L, mut jobs: mpsc::Receiver<(usize, Fut)>) -> Results<O>
where
    L: Limiter + Send + 'static,
    Fut: Future<Output = O> + Send + 'static,
    O: Send + 'static,
{
    let (tx, rx) = mpsc::channel::<(usize, O)>(RESULT_BUFFER);
    tokio::spawn(async move {
        let mut set: JoinSet<()> = JoinSet::new();
        loop {
            tokio::select! {
                biased;
                () = tx.closed() => {
                    // Consumer dropped the stream: abort in-flight jobs and stop admission.
                    set.shutdown().await;
                    return;
                }
                maybe = jobs.recv() => match maybe {
                    Some((id, fut)) => {
                        let permit = limiter.acquire().await;
                        let out = tx.clone();
                        set.spawn(async move {
                            let _permit = permit;
                            let _ = out.send((id, fut.await)).await;
                        });
                    }
                    None => break,
                },
            }
        }
        // Input drained: let in-flight jobs finish and emit their results, but keep
        // watching for the consumer dropping the stream mid-drain so a late drop
        // still aborts jobs still in flight.
        loop {
            tokio::select! {
                biased;
                () = tx.closed() => {
                    set.shutdown().await;
                    return;
                }
                joined = set.join_next() => {
                    if joined.is_none() {
                        return;
                    }
                }
            }
        }
    });
    Results { rx }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
    use std::time::Duration;

    use futures_util::StreamExt;
    use tokio::sync::{Barrier, OwnedSemaphorePermit, Semaphore, oneshot};

    use super::drive;
    use crate::agent::orchestration::core::limiter::Limiter;
    use crate::agent::orchestration::limit::semaphore::SemaphoreLimiter;

    /// A limiter that records the peak number of concurrently-held permits.
    #[derive(Clone)]
    struct Gauge {
        semaphore: Arc<Semaphore>,
        live: Arc<AtomicUsize>,
        peak: Arc<AtomicUsize>,
    }

    impl Gauge {
        fn new(permits: usize) -> Self {
            Self {
                semaphore: Arc::new(Semaphore::new(permits)),
                live: Arc::new(AtomicUsize::new(0)),
                peak: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    struct GaugePermit {
        _inner: OwnedSemaphorePermit,
        live: Arc<AtomicUsize>,
    }

    impl Drop for GaugePermit {
        fn drop(&mut self) {
            self.live.fetch_sub(1, SeqCst);
        }
    }

    impl Limiter for Gauge {
        type Permit = GaugePermit;

        async fn acquire(&self) -> Self::Permit {
            let inner = self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("gauge semaphore open");
            let now = self.live.fetch_add(1, SeqCst) + 1;
            self.peak.fetch_max(now, SeqCst);
            GaugePermit {
                _inner: inner,
                live: self.live.clone(),
            }
        }
    }

    #[tokio::test]
    async fn runs_all_jobs_and_tags_each_output_with_its_id() {
        let limiter = SemaphoreLimiter::new(4);
        let (jtx, jrx) = tokio::sync::mpsc::channel(8);
        for i in 0..8usize {
            let _ = jtx.try_send((i, async move { i * 10 }));
        }
        drop(jtx);

        let mut out = drive(limiter, jrx);
        let mut pairs = Vec::new();
        while let Some((id, value)) = out.next().await {
            pairs.push((id, value));
        }
        pairs.sort_unstable();
        let expected: Vec<(usize, usize)> = (0..8).map(|i| (i, i * 10)).collect();
        assert_eq!(pairs, expected);
    }

    #[tokio::test]
    async fn never_exceeds_the_limiter_bound() {
        let n = 3usize;
        let total = 9usize;
        let limiter = Gauge::new(n);
        let peak = limiter.peak.clone();
        // Barrier(n) forces exactly n jobs to be live simultaneously per wave;
        // with n permits this proves peak reaches n and (by acquire-before-spawn)
        // never exceeds it.
        let barrier = Arc::new(Barrier::new(n));
        let (jtx, jrx) = tokio::sync::mpsc::channel(total);
        for i in 0..total {
            let gate = barrier.clone();
            let _ = jtx.try_send((i, async move {
                gate.wait().await;
                i
            }));
        }
        drop(jtx);

        let mut out = drive(limiter, jrx);
        let mut count = 0usize;
        while let Some((id, value)) = out.next().await {
            assert_eq!(id, value);
            count += 1;
        }
        assert_eq!(count, total);
        assert_eq!(peak.load(SeqCst), n);
    }

    #[tokio::test]
    async fn emits_in_completion_order_not_submission_order() {
        let limiter = SemaphoreLimiter::new(4);
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let (jtx, jrx) = tokio::sync::mpsc::channel(2);
        for (id, rx) in [(0usize, rx0), (1usize, rx1)] {
            let _ = jtx.try_send((id, async move {
                let _ = rx.await;
                id
            }));
        }
        drop(jtx);

        let mut out = drive(limiter, jrx);
        // Release the second-submitted job first; it must be emitted first.
        let _ = tx1.send(());
        assert_eq!(out.next().await, Some((1, 1)));
        let _ = tx0.send(());
        assert_eq!(out.next().await, Some((0, 0)));
        assert!(out.next().await.is_none());
    }

    #[tokio::test]
    async fn dropping_the_stream_aborts_in_flight_jobs() {
        struct Guard(Arc<AtomicUsize>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_add(1, SeqCst);
            }
        }

        let limiter = SemaphoreLimiter::new(4);
        let dropped = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let (_never_tx, never_rx) = oneshot::channel::<()>();
        let (jtx, jrx) = tokio::sync::mpsc::channel(1);
        let guard_flag = dropped.clone();
        let done_flag = completed.clone();
        let _ = jtx.try_send((0usize, async move {
            let _guard = Guard(guard_flag);
            let _ = never_rx.await; // parks forever (sender never fires)
            done_flag.fetch_add(1, SeqCst);
            0usize
        }));
        drop(jtx);

        let out = drive(limiter, jrx);
        // Let the job start and park on the oneshot.
        tokio::time::sleep(Duration::from_millis(10)).await;
        drop(out); // consumer cancels
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(completed.load(SeqCst), 0, "job never ran to completion");
        assert_eq!(
            dropped.load(SeqCst),
            1,
            "in-flight job future was aborted (dropped)"
        );
    }
}
