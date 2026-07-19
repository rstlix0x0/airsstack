//! Error hierarchy for the Agent SDK surface.
//!
//! `AgentError` is the single error type crossing the public Agent API. It
//! wraps the protocol-blind [`ProcessError`] from the subprocess layer and
//! adds the protocol-, discovery-, and control-level failure modes that the
//! `CliRuntime` and codec can raise.

use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

use crate::agent::process::ProcessError;

/// All failure modes surfaced across the public Agent SDK API.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AgentError {
    /// The `claude` binary could not be located.
    #[error("claude binary not found; searched: {searched:?}")]
    BinaryNotFound {
        /// Paths inspected during discovery, in order.
        searched: Vec<PathBuf>,
    },
    /// The located binary is older than the required minimum.
    #[error("claude binary version {found} is below required minimum {minimum}")]
    BinaryVersionUnsupported {
        /// Version reported by the located binary.
        found: String,
        /// Minimum version the SDK requires.
        minimum: String,
    },
    /// The subprocess layer failed (spawn, kill, reap, timeout).
    #[error(transparent)]
    Process(#[from] ProcessError),
    /// A protocol-level violation: malformed framing or an unexpected frame.
    #[error("protocol error: {detail}")]
    Protocol {
        /// Human-readable description of the violation.
        detail: String,
    },
    /// A frame body failed to deserialize.
    #[error("failed to decode frame: {0}")]
    Decode(String),
    /// A control request returned an error response from the binary.
    #[error("control request `{method}` failed: {detail}")]
    ControlRequestFailed {
        /// The control subtype that failed (e.g. `interrupt`).
        method: String,
        /// Error detail reported by the binary.
        detail: String,
    },
    /// The subprocess stdout/stdin channel closed unexpectedly.
    #[error("transport closed before the operation completed")]
    TransportClosed,
    /// A control request's correlated response did not arrive within the
    /// configured [`control_request_timeout`](crate::agent::options::Options::control_request_timeout)
    /// bound.
    #[error("control request `{method}` timed out after {elapsed:?}")]
    ControlRequestTimedOut {
        /// The control subtype that timed out (e.g. `interrupt`).
        method: String,
        /// The configured timeout bound that expired (not measured elapsed
        /// time — the caller waited at least this long).
        elapsed: Duration,
    },
    /// The binary exited nonzero or on a signal.
    #[error("claude exited with status {exit_code:?}: {stderr}")]
    Cli {
        /// Exit code if the process exited normally; `None` on signal.
        exit_code: Option<i32>,
        /// Captured stderr tail.
        stderr: String,
    },
    /// A requested feature is absent from the negotiated capabilities.
    #[error("capability `{feature}` is not supported by this binary")]
    CapabilityUnsupported {
        /// The unsupported feature name.
        feature: String,
    },
    /// The session was interrupted.
    #[error("operation interrupted")]
    Interrupted,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::AgentError;
    use crate::agent::process::ProcessError;

    #[test]
    fn process_error_converts_via_from() {
        let err: AgentError = ProcessError::Timeout.into();
        assert!(matches!(err, AgentError::Process(ProcessError::Timeout)));
    }

    #[test]
    fn binary_not_found_displays_searched_paths() {
        let err = AgentError::BinaryNotFound {
            searched: vec!["/usr/bin".into(), "/opt".into()],
        };
        let shown = err.to_string();
        assert!(shown.contains("/usr/bin"), "got: {shown}");
    }

    #[test]
    fn control_request_timed_out_displays_method_and_elapsed() {
        let err = AgentError::ControlRequestTimedOut {
            method: "interrupt".to_string(),
            elapsed: Duration::from_secs(60),
        };
        let shown = err.to_string();
        assert!(shown.contains("interrupt"), "got: {shown}");
        assert!(shown.contains("60s"), "got: {shown}");
    }
}
