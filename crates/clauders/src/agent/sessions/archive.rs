//! The `SessionArchive` handle over a config root, and the five
//! session-file operations it exposes.

use std::path::PathBuf;

use super::error::SessionError;
use super::path::{projects_root, resolve_config_root};

/// A handle to the on-disk session store rooted at a config directory.
///
/// Stateless and cheap to clone — it holds only the config-root path. The
/// operations read and append to `<base>/projects/<encoded-cwd>/<id>.jsonl`.
#[derive(Clone, Debug)]
pub struct SessionArchive {
    base: PathBuf,
}

impl SessionArchive {
    /// Build an archive over the default config root
    /// (`CLAUDE_CONFIG_DIR`, else `$HOME/.claude`).
    ///
    /// # Errors
    /// Returns [`SessionError::NoConfigRoot`] when neither `CLAUDE_CONFIG_DIR`
    /// nor `HOME` is set.
    pub fn new() -> Result<Self, SessionError> {
        Ok(Self {
            base: resolve_config_root()?,
        })
    }

    /// Build an archive over an explicit config root (the directory that
    /// contains `projects/`). Primarily for tests and non-default installs.
    pub fn with_base(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    /// The `projects/` directory this archive reads from.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "will back the five session-file operations once those are added"
        )
    )]
    pub(crate) fn projects(&self) -> PathBuf {
        projects_root(&self.base)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn with_base_points_projects_under_it() {
        let a = SessionArchive::with_base("/x/.claude");
        assert_eq!(a.projects(), Path::new("/x/.claude/projects"));
    }
}
