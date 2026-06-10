use thiserror::Error;

/// Failure modes of the subprocess-management layer.
///
/// I/O error payloads are captured as their formatted string so the error
/// type stays `Clone` (the supervisor publishes its outcome to multiple
/// awaiters through a `watch` channel).
#[derive(Debug, Clone, Error)]
pub enum ProcessError {
    /// The child process could not be spawned.
    #[error("failed to spawn process: {0}")]
    Spawn(String),
    /// Killing the child (or its process group) failed.
    #[error("failed to kill process: {0}")]
    Kill(String),
    /// The child did not exit within the shutdown grace period even after
    /// a forced kill.
    #[error("process did not exit within the shutdown grace period")]
    Timeout,
    /// The process has already been torn down; no exit status is available.
    #[error("process has already been shut down")]
    AlreadyShutDown,
}
