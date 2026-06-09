//! Protocol-blind subprocess management.
//!
//! Spawns an arbitrary child process, owns its pipes, and tears it down
//! with a zombie/orphan-safe lifecycle. Nothing here knows about the
//! `claude` binary or the control protocol.

mod error;
mod guard;
mod pipes;
mod spawn;
mod supervisor;

use std::io;
use std::process::ExitStatus;
use std::sync::Arc;

use tokio::process::ChildStdin;

pub use error::ProcessError;
pub use pipes::{StderrBuffer, StdoutLines};
pub use spawn::ProcessConfig;

use spawn::build_command;
use supervisor::Supervisor;

/// The pipe ends of a spawned child, handed to the caller.
///
/// Dropping [`ProcessIo::stdin`] sends EOF to the child, which is the
/// primary graceful-shutdown signal for well-behaved children.
pub struct ProcessIo {
    /// Writable stdin of the child.
    pub stdin: ChildStdin,
    /// Line-oriented stdout reader.
    pub stdout: StdoutLines,
    /// Continuously-drained, bounded stderr capture.
    pub stderr: StderrBuffer,
}

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

fn missing(stream: &str) -> String {
    io::Error::other(format!("{stream} was not captured")).to_string()
}
