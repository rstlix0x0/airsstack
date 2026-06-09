use std::sync::Arc;
use std::time::Duration;

use tokio::process::Child;
use tokio::sync::{Notify, watch};
use tokio::time::timeout;

use super::error::ProcessError;

type Outcome = Result<std::process::ExitStatus, ProcessError>;

/// Owns the spawned `Child` for its whole life and is the single site that
/// calls `wait()`. Teardown is requested via `shutdown` (a `Notify`); the
/// final outcome is published once on `result_rx`.
pub(super) struct Supervisor {
    pid: Option<u32>,
    shutdown: Arc<Notify>,
    result_rx: watch::Receiver<Option<Outcome>>,
}

impl Supervisor {
    /// Take ownership of `child` and spawn the detached supervisor task.
    pub(super) fn spawn(child: Child, grace: Duration) -> Self {
        let pid = child.id();
        let shutdown = Arc::new(Notify::new());
        let (result_tx, result_rx) = watch::channel(None);
        let task_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            let outcome = run(child, grace, task_shutdown).await;
            let _ = result_tx.send(Some(outcome));
        });
        Self {
            pid,
            shutdown,
            result_rx,
        }
    }

    pub(super) const fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Request graceful teardown, then await the outcome.
    pub(super) async fn shutdown(&self) -> Outcome {
        self.shutdown.notify_one();
        self.result().await
    }

    /// Await the outcome without requesting teardown (natural exit).
    pub(super) async fn wait(&self) -> Outcome {
        self.result().await
    }

    async fn result(&self) -> Outcome {
        let mut rx = self.result_rx.clone();
        loop {
            let current = rx.borrow().clone();
            if let Some(outcome) = current {
                return outcome;
            }
            if rx.changed().await.is_err() {
                return Err(ProcessError::AlreadyShutDown);
            }
        }
    }
}

/// Drive the child to completion. Graceful path only: this layer waits for
/// natural exit or, on shutdown request, waits out the grace window; it never
/// force-kills.
async fn run(mut child: Child, grace: Duration, shutdown: Arc<Notify>) -> Outcome {
    tokio::select! {
        result = child.wait() => result.map_err(|e| ProcessError::Kill(e.to_string())),
        () = shutdown.notified() => {
            match timeout(grace, child.wait()).await {
                Ok(result) => result.map_err(|e| ProcessError::Kill(e.to_string())),
                Err(_elapsed) => Err(ProcessError::Timeout),
            }
        }
    }
}
