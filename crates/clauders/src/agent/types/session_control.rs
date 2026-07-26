//! Session continuation intent for a runtime.

use crate::agent::types::{MessageId, SessionId};

/// How a runtime should treat conversation history for a session.
///
/// Maps to the backend's session flags on the CLI runtime, and to the
/// native history store on the API runtime. `New` is the default: a fresh
/// session with no prior turns.
///
/// # Examples
///
/// ```
/// use clauders::agent::SessionControl;
/// assert_eq!(SessionControl::default(), SessionControl::New);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SessionControl {
    /// Start a fresh session with no prior history.
    #[default]
    New,
    /// Continue the most recent session, optionally forking it into a new one.
    Continue {
        /// Branch into a new session instead of continuing in place.
        fork: bool,
    },
    /// Resume a specific session by id, optionally forking it into a new one.
    Resume {
        /// The session to resume.
        id: SessionId,
        /// Branch into a new session instead of continuing in place.
        fork: bool,
        /// Resume only the messages up to this message uuid.
        ///
        /// Lowers to the binary's `--resume-session-at=<uuid>`, which requires
        /// `--resume`; keeping it on this variant makes that requirement
        /// unrepresentable when not resuming.
        resume_at: Option<MessageId>,
    },
}

#[cfg(test)]
mod tests {
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::SessionControl;
    use crate::agent::types::SessionId;

    #[test]
    fn default_is_new() {
        assert_eq!(SessionControl::default(), SessionControl::New);
    }

    #[test]
    fn continue_carries_fork_flag() {
        assert_eq!(
            SessionControl::Continue { fork: true },
            SessionControl::Continue { fork: true }
        );
        assert_ne!(
            SessionControl::Continue { fork: true },
            SessionControl::Continue { fork: false }
        );
    }

    #[test]
    fn resume_carries_id_and_fork() {
        let control = SessionControl::Resume {
            id: SessionId::new("sess_abc"),
            fork: false,
            resume_at: None,
        };
        match control {
            SessionControl::Resume {
                id,
                fork,
                resume_at,
            } => {
                assert_eq!(id.as_str(), "sess_abc");
                assert!(!fork);
                assert!(resume_at.is_none());
            }
            other => panic!("expected Resume, got {other:?}"),
        }
    }
}
