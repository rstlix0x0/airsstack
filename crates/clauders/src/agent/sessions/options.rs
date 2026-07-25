//! Options for the listing and message-reading operations.

use std::path::PathBuf;

/// Options for `SessionArchive::list` (the listing operation).
#[derive(Clone, Debug)]
pub struct ListOptions {
    /// Scope the listing to sessions for this working directory. `None`
    /// scans every project directory.
    pub dir: Option<PathBuf>,
    /// Maximum number of sessions to return. `None` is unbounded.
    pub limit: Option<usize>,
    /// Number of leading sessions to skip.
    pub offset: usize,
    /// Include sessions from git worktrees linked to `dir`.
    pub include_worktrees: bool,
    /// Include programmatic (SDK-initiated / daemon) sessions.
    pub include_programmatic: bool,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            dir: None,
            limit: None,
            offset: 0,
            include_worktrees: true,
            include_programmatic: true,
        }
    }
}

/// Options for `SessionArchive::messages` (the message-reading operation).
#[derive(Clone, Debug, Default)]
pub struct MessagesOptions {
    /// Scope the lookup to this working directory (as for `list`).
    pub dir: Option<PathBuf>,
    /// Include `system` messages (default: user/assistant only).
    pub include_system_messages: bool,
    /// Maximum number of messages to return. `None` is unbounded.
    pub limit: Option<usize>,
    /// Number of leading messages to skip.
    pub offset: usize,
}

#[cfg(test)]
mod tests {
    use super::{ListOptions, MessagesOptions};

    #[test]
    fn defaults_match_the_binary() {
        let o = ListOptions::default();
        assert!(o.dir.is_none());
        assert!(o.limit.is_none());
        assert_eq!(o.offset, 0);
        assert!(o.include_worktrees);
        assert!(o.include_programmatic);
    }

    #[test]
    fn messages_options_default_scopes_nothing_and_excludes_system() {
        let o = MessagesOptions::default();
        assert!(o.dir.is_none());
        assert!(!o.include_system_messages);
        assert!(o.limit.is_none());
        assert_eq!(o.offset, 0);
    }
}
