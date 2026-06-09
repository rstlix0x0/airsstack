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

    /// Signal teardown without awaiting the outcome.
    ///
    /// Called from `Drop`, which is synchronous, so it can only fire the
    /// `Notify` here; the detached supervisor task performs the async
    /// graceful→kill→reap sequence.
    pub(super) fn request_shutdown(self_: &Arc<Self>) {
        self_.shutdown.notify_one();
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

/// Drive the child to completion.
///
/// On a shutdown request, waits up to `grace` for the child to exit on its
/// own. If the child is still running after that window, the entire process
/// group (on Unix) is sent a forced kill; on other platforms the direct child
/// is killed. A second `grace` window is then given for the forced-kill reap.
/// Natural exit (no shutdown requested) returns immediately when the child
/// exits.
async fn run(mut child: Child, grace: Duration, shutdown: Arc<Notify>) -> Outcome {
    tokio::select! {
        result = child.wait() => result.map_err(|e| ProcessError::Kill(e.to_string())),
        () = shutdown.notified() => {
            match timeout(grace, child.wait()).await {
                Ok(result) => result.map_err(|e| ProcessError::Kill(e.to_string())),
                Err(_elapsed) => {
                    kill_tree(&mut child)?;
                    match timeout(grace, child.wait()).await {
                        Ok(result) => result.map_err(|e| ProcessError::Kill(e.to_string())),
                        Err(_elapsed) => Err(ProcessError::Timeout),
                    }
                }
            }
        }
    }
}

/// Forcibly kill the child. On Unix this signals the entire process group
/// (so the child's own descendants die too); elsewhere it kills just the
/// direct child. The pgid is read from the live `Child` at call time — never
/// a stored value — so a recycled pid can never be signalled.
#[cfg(unix)]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "non-unix cfg branch calls start_kill(&mut self); keeping the \
              signature consistent across platforms avoids a signature mismatch"
)]
fn kill_tree(child: &mut Child) -> Result<(), ProcessError> {
    use nix::errno::Errno;
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;

    let Some(pid) = child.id() else {
        return Ok(());
    };
    let Ok(raw) = i32::try_from(pid) else {
        return Ok(());
    };
    match killpg(Pid::from_raw(raw), Signal::SIGKILL) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(err) => Err(ProcessError::Kill(err.to_string())),
    }
}

#[cfg(not(unix))]
fn kill_tree(child: &mut Child) -> Result<(), ProcessError> {
    child
        .start_kill()
        .map_err(|e| ProcessError::Kill(e.to_string()))
}
