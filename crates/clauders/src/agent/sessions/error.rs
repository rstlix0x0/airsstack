//! Error type for session-file operations.

use std::io;

use thiserror::Error;

/// Failure modes of the session-file operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SessionError {
    /// The config root could not be resolved (no `CLAUDE_CONFIG_DIR` and no `HOME`).
    #[error("cannot resolve the config root: neither CLAUDE_CONFIG_DIR nor HOME is set")]
    NoConfigRoot,
    /// A filesystem operation failed.
    #[error("session I/O failed: {0}")]
    Io(#[from] io::Error),
    /// A transcript line or metadata value could not be parsed.
    #[error("failed to parse session data: {0}")]
    Parse(String),
    /// The session id is not a valid UUID (rename/tag reject these).
    #[error("invalid session id: {0}")]
    InvalidSessionId(String),
    /// A rename title or tag value was empty after trimming.
    #[error("{0}")]
    EmptyValue(String),
    /// The target session file was not found for a rename/tag append.
    #[error("session {session_id} not found{location}")]
    SessionNotFound {
        /// The session id that could not be located.
        session_id: String,
        /// Where it was looked for (e.g. " in project directory for /repo").
        location: String,
    },
}

#[cfg(test)]
mod tests {
    use super::SessionError;

    #[test]
    fn invalid_session_id_displays_the_id() {
        let e = SessionError::InvalidSessionId("not-a-uuid".to_string());
        assert!(e.to_string().contains("not-a-uuid"), "got: {e}");
    }

    #[test]
    fn session_not_found_displays_id_and_location() {
        let e = SessionError::SessionNotFound {
            session_id: "abc".to_string(),
            location: " in any project directory".to_string(),
        };
        let s = e.to_string();
        assert!(
            s.contains("abc") && s.contains("any project directory"),
            "got: {s}"
        );
    }
}
