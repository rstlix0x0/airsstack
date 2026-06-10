//! Owned handle to a supervised child process with shutdown/wait control.
//!
//! Exists as its own module so `ManagedProcess` — including its `Drop` impl —
//! lives in one place. Collocating `Drop` with the struct and its `impl` block
//! is required because `Drop` accesses the private `supervisor` field; a
//! sibling file cannot name private fields of a type defined elsewhere.
//!
//! Responsibilities:
//! - Define [`ManagedProcess`] and its private `supervisor` field.
//! - Implement `spawn`, `shutdown`, `wait`, and `id` on `ManagedProcess`.
//! - Implement `Drop` for `ManagedProcess` so a dropped handle never orphans
//!   the child (signals the detached supervisor task via `Notify`).
//!
//! Not responsible for:
//! - The async teardown sequence — that lives in `supervisor`.
//! - Building the `tokio::process::Command` — that lives in `spawn`.
//! - Owning pipe ends — those are in `io::ProcessIo` and returned to the caller.

use std::io;
use std::process::ExitStatus;
use std::sync::Arc;

use super::error::ProcessError;
use super::io::ProcessIo;
use super::pipes::{StderrBuffer, StdoutLines};
use super::spawn::{ProcessConfig, build_command};
use super::supervisor::Supervisor;

/// A handle to a spawned, supervised child process.
///
/// Lifecycle is owned by a detached supervisor task; this handle exposes
/// `shutdown`/`wait`/`id`. Dropping the handle requests teardown so a child
/// can never be orphaned.
pub struct ManagedProcess {
    supervisor: Arc<Supervisor>,
}

impl ManagedProcess {
    /// Spawn `cfg` and return the handle plus its pipe ends.
    ///
    /// # Errors
    /// Returns [`ProcessError::Spawn`] if the process cannot be started or
    /// any standard stream was not captured.
    pub fn spawn(cfg: &ProcessConfig) -> Result<(Self, ProcessIo), ProcessError> {
        let grace = cfg.shutdown_grace;
        let mut command = build_command(cfg);
        let mut child = command
            .spawn()
            .map_err(|e| ProcessError::Spawn(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ProcessError::Spawn(missing("stdin")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProcessError::Spawn(missing("stdout")))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ProcessError::Spawn(missing("stderr")))?;

        let io = ProcessIo {
            stdin,
            stdout: StdoutLines::new(stdout),
            stderr: StderrBuffer::drain(stderr),
        };
        let supervisor = Arc::new(Supervisor::spawn(child, grace));
        Ok((Self { supervisor }, io))
    }

    /// Request graceful teardown and return the exit status.
    ///
    /// Waits up to the configured grace period for the child to exit on its
    /// own. If the child is still running after that window, the supervisor
    /// escalates to a forced (group, on Unix) kill and waits for the reap.
    ///
    /// # Errors
    /// Returns a [`ProcessError`] if the kill or subsequent wait fails, or if
    /// the child still has not exited after the post-kill grace window.
    pub async fn shutdown(&self) -> Result<ExitStatus, ProcessError> {
        self.supervisor.shutdown().await
    }

    /// Await the child's natural exit (no teardown requested).
    ///
    /// # Errors
    /// Returns a [`ProcessError`] if waiting fails.
    pub async fn wait(&self) -> Result<ExitStatus, ProcessError> {
        self.supervisor.wait().await
    }

    /// The child's OS process id while it is still running.
    #[must_use]
    pub fn id(&self) -> Option<u32> {
        self.supervisor.pid()
    }
}

impl Drop for ManagedProcess {
    /// Request teardown so a dropped handle never orphans the child.
    ///
    /// `Drop` is synchronous, so it only *signals* the detached supervisor
    /// task (via `Notify`); that task performs the async graceful→kill→reap
    /// sequence. If the runtime is already gone, `kill_on_drop(true)` on the
    /// spawn command is the final SIGKILL safety net.
    fn drop(&mut self) {
        Supervisor::request_shutdown(&self.supervisor);
    }
}

fn missing(stream: &str) -> String {
    io::Error::other(format!("{stream} was not captured")).to_string()
}
