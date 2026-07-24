//! The `SessionArchive` handle over a config root, and the five
//! session-file operations it exposes.

use std::path::PathBuf;

use super::error::SessionError;
use super::info::{SessionInfo, build_info, read_head_tail};
use super::path::{find_session_file, is_session_id, projects_root, resolve_config_root};

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
    pub(crate) fn projects(&self) -> PathBuf {
        projects_root(&self.base)
    }

    /// Metadata for one session, or `None` when the id is not a UUID or no
    /// such session exists.
    ///
    /// # Errors
    /// Returns [`SessionError`] on a genuine filesystem read failure.
    pub async fn info(
        &self,
        session_id: &str,
        dir: Option<&str>,
    ) -> Result<Option<SessionInfo>, SessionError> {
        if !is_session_id(session_id) {
            return Ok(None);
        }
        let root = self.projects();
        let Some(found) = find_session_file(&root, session_id, dir).await else {
            return Ok(None);
        };
        let Some(ht) = read_head_tail(&found.file_path).await? else {
            return Ok(None);
        };
        Ok(build_info(session_id, &ht, found.project_path.as_deref()))
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::path::Path;

    use super::*;

    #[test]
    fn with_base_points_projects_under_it() {
        let a = SessionArchive::with_base("/x/.claude");
        assert_eq!(a.projects(), Path::new("/x/.claude/projects"));
    }

    #[tokio::test]
    async fn info_returns_none_for_a_bad_uuid() {
        let tmp = tempfile::tempdir().expect("tmp");
        let a = SessionArchive::with_base(tmp.path());
        assert!(a.info("not-a-uuid", None).await.expect("ok").is_none());
    }

    #[tokio::test]
    async fn info_returns_none_when_absent() {
        let tmp = tempfile::tempdir().expect("tmp");
        let a = SessionArchive::with_base(tmp.path());
        assert!(
            a.info("f28ced56-9bd4-41f8-a37d-2a496c7d0e35", None)
                .await
                .expect("ok")
                .is_none()
        );
    }

    #[tokio::test]
    async fn info_reads_a_present_session() {
        let tmp = tempfile::tempdir().expect("tmp");
        let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
        let dir = tmp.path().join("projects").join("-repo-a");
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        let line = r#"{"type":"user","timestamp":"2026-07-23T09:37:06.000Z","cwd":"/repo","message":{"content":"the prompt"}}"#;
        tokio::fs::write(dir.join(format!("{id}.jsonl")), format!("{line}\n"))
            .await
            .expect("write");
        let a = SessionArchive::with_base(tmp.path());
        let info = a.info(id, None).await.expect("ok").expect("some");
        assert_eq!(info.session_id.as_str(), id);
        assert_eq!(info.summary, "the prompt");
    }
}
