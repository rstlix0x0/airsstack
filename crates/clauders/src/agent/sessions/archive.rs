//! The `SessionArchive` handle over a config root, and the five
//! session-file operations it exposes.

use std::path::PathBuf;

use super::error::SessionError;
use super::info::{
    SessionEntry, SessionInfo, build_info, collect_paged, collect_unpaged, list_dir_entries,
    read_head_tail,
};
use super::message::{
    SessionMessage, assemble_messages, dedup_by_uuid, encode_rename_record, encode_tag_record,
    parse_transcript,
};
use super::options::{ListOptions, MessagesOptions};
use super::path::{
    append_session_record, candidate_dirs, candidate_dirs_with_worktrees, find_session_file,
    is_session_id, projects_root, resolve_config_root,
};

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

    /// List stored sessions as metadata records, newest first.
    ///
    /// # Errors
    /// Returns [`SessionError`] only on a failure resolving the projects
    /// root; per-file read failures are skipped, matching the binary.
    pub async fn list(&self, opts: ListOptions) -> Result<Vec<SessionInfo>, SessionError> {
        let root = self.projects();
        let paged = opts.limit.is_some_and(|l| l > 0) || opts.offset > 0;
        let entries = if let Some(dir) = opts.dir.as_deref() {
            let cwd = dir.to_string_lossy().into_owned();
            let mut acc: Vec<SessionEntry> = Vec::new();
            let dirs = if opts.include_worktrees {
                candidate_dirs_with_worktrees(&root, &cwd).await
            } else {
                candidate_dirs(&root, &cwd)
                    .await
                    .into_iter()
                    .map(|d| (d, cwd.clone()))
                    .collect()
            };
            for (candidate, wt) in dirs {
                acc.extend(list_dir_entries(&candidate, paged, Some(&wt)).await);
            }
            acc
        } else {
            let mut acc: Vec<SessionEntry> = Vec::new();
            if let Ok(mut rd) = tokio::fs::read_dir(&root).await {
                while let Ok(Some(sub)) = rd.next_entry().await {
                    if sub.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                        acc.extend(list_dir_entries(&sub.path(), paged, None).await);
                    }
                }
            }
            acc
        };
        Ok(if paged {
            collect_paged(entries, opts.limit, opts.offset, opts.include_programmatic).await
        } else {
            collect_unpaged(entries, opts.include_programmatic).await
        })
    }

    /// Read a session's messages as the full flat transcript (user/assistant,
    /// and system when requested), deduped by uuid and in file order, each
    /// carrying its `parent_uuid`. A bad uuid or absent session yields an
    /// empty vector.
    ///
    /// This deliberately differs from the official SDK, which returns a
    /// reconstructed single active thread; clauders returns the full
    /// transcript and leaves branch reconstruction to the caller.
    ///
    /// # Errors
    /// Returns [`SessionError`] on a genuine read failure.
    pub async fn messages(
        &self,
        session_id: &str,
        opts: MessagesOptions,
    ) -> Result<Vec<SessionMessage>, SessionError> {
        if !is_session_id(session_id) {
            return Ok(Vec::new());
        }
        let root = self.projects();
        let dir = opts.dir.as_ref().map(|p| p.to_string_lossy().into_owned());
        let Some(found) = find_session_file(&root, session_id, dir.as_deref()).await else {
            return Ok(Vec::new());
        };
        let bytes = tokio::fs::read(&found.file_path).await?;
        let entries = dedup_by_uuid(parse_transcript(&bytes));
        Ok(assemble_messages(
            entries,
            opts.include_system_messages,
            opts.limit,
            opts.offset,
        ))
    }

    /// Set a session's custom title by appending a title record.
    ///
    /// # Errors
    /// [`SessionError::InvalidSessionId`] for a non-UUID id;
    /// [`SessionError::EmptyValue`] for a blank title;
    /// [`SessionError::SessionNotFound`] when no such session file exists.
    pub async fn rename(
        &self,
        session_id: &str,
        title: &str,
        dir: Option<&str>,
    ) -> Result<(), SessionError> {
        if !is_session_id(session_id) {
            return Err(SessionError::InvalidSessionId(session_id.to_string()));
        }
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(SessionError::EmptyValue(
                "title must be non-empty".to_string(),
            ));
        }
        let record = encode_rename_record(session_id, trimmed);
        append_session_record(&self.projects(), session_id, dir, &record).await
    }

    /// Set or clear a session's tag. `Some(tag)` sets it (trimmed);
    /// `None` clears it (an empty-tag record the reader treats as absent).
    ///
    /// # Errors
    /// [`SessionError::InvalidSessionId`] for a non-UUID id;
    /// [`SessionError::EmptyValue`] when `Some` trims to empty;
    /// [`SessionError::SessionNotFound`] when no such session file exists.
    pub async fn tag(
        &self,
        session_id: &str,
        tag: Option<&str>,
        dir: Option<&str>,
    ) -> Result<(), SessionError> {
        if !is_session_id(session_id) {
            return Err(SessionError::InvalidSessionId(session_id.to_string()));
        }
        let value = match tag {
            Some(t) => {
                let trimmed = t.trim();
                if trimmed.is_empty() {
                    return Err(SessionError::EmptyValue(
                        "tag must be non-empty (use None to clear)".to_string(),
                    ));
                }
                trimmed.to_string()
            }
            None => String::new(),
        };
        let record = encode_tag_record(session_id, &value);
        append_session_record(&self.projects(), session_id, dir, &record).await
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

    #[tokio::test]
    async fn list_returns_sessions_across_all_project_dirs() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("projects");
        for (dir, id) in [
            ("-repo-a", "a0000000-0000-4000-8000-000000000001"),
            ("-repo-b", "a0000000-0000-4000-8000-000000000002"),
        ] {
            let d = root.join(dir);
            tokio::fs::create_dir_all(&d).await.expect("mkdir");
            tokio::fs::write(
                d.join(format!("{id}.jsonl")),
                b"{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\n",
            )
            .await
            .expect("write");
        }
        let a = SessionArchive::with_base(tmp.path());
        let got = a.list(ListOptions::default()).await.expect("ok");
        assert_eq!(got.len(), 2);
    }

    #[tokio::test]
    async fn messages_returns_empty_for_bad_uuid_or_absent() {
        let tmp = tempfile::tempdir().expect("tmp");
        let a = SessionArchive::with_base(tmp.path());
        assert!(
            a.messages("not-a-uuid", MessagesOptions::default())
                .await
                .expect("ok")
                .is_empty()
        );
        assert!(
            a.messages(
                "f28ced56-9bd4-41f8-a37d-2a496c7d0e35",
                MessagesOptions::default()
            )
            .await
            .expect("ok")
            .is_empty()
        );
    }

    #[tokio::test]
    async fn messages_returns_the_full_flat_transcript_with_parent_uuid() {
        let tmp = tempfile::tempdir().expect("tmp");
        let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
        let dir = tmp.path().join("projects").join("-repo-a");
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        // A forked session: u1 -> a1, and u1 -> a2 (a second branch). The flat
        // contract returns ALL of them (unlike the TS active-thread reader).
        let jsonl = concat!(
            r#"{"type":"user","uuid":"u1","parentUuid":null,"sessionId":"s","message":{"role":"user","content":"hi"}}"#,
            "\n",
            r#"{"type":"assistant","uuid":"a1","parentUuid":"u1","sessionId":"s","message":{"id":"m1","content":[{"type":"text","text":"one"}]}}"#,
            "\n",
            r#"{"type":"assistant","uuid":"a2","parentUuid":"u1","sessionId":"s","message":{"id":"m2","content":[{"type":"text","text":"two"}]}}"#,
            "\n",
        );
        tokio::fs::write(dir.join(format!("{id}.jsonl")), jsonl)
            .await
            .expect("write");
        let a = SessionArchive::with_base(tmp.path());
        let msgs = a
            .messages(id, MessagesOptions::default())
            .await
            .expect("ok");
        let ids: Vec<&str> = msgs.iter().map(|m| m.uuid.as_str()).collect();
        assert_eq!(
            ids,
            vec!["u1", "a1", "a2"],
            "both branches returned, in file order"
        );
        assert_eq!(msgs[1].parent_uuid.as_deref(), Some("u1"));
        assert_eq!(msgs[2].parent_uuid.as_deref(), Some("u1"));
    }

    #[tokio::test]
    async fn rename_rejects_bad_uuid_and_empty_title() {
        let tmp = tempfile::tempdir().expect("tmp");
        let a = SessionArchive::with_base(tmp.path());
        assert!(matches!(
            a.rename("bad", "t", None).await,
            Err(SessionError::InvalidSessionId(_))
        ));
        assert!(matches!(
            a.rename("f28ced56-9bd4-41f8-a37d-2a496c7d0e35", "   ", None)
                .await,
            Err(SessionError::EmptyValue(_))
        ));
    }

    #[tokio::test]
    async fn rename_then_info_reflects_the_title() {
        let tmp = tempfile::tempdir().expect("tmp");
        let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
        let dir = tmp.path().join("projects").join("-repo-a");
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        tokio::fs::write(
            dir.join(format!("{id}.jsonl")),
            b"{\"type\":\"user\",\"message\":{\"content\":\"q\"}}\n",
        )
        .await
        .expect("write");
        let a = SessionArchive::with_base(tmp.path());
        a.rename(id, "Renamed", None).await.expect("rename");
        let info = a.info(id, None).await.expect("ok").expect("some");
        assert_eq!(info.custom_title.as_deref(), Some("Renamed"));
        assert_eq!(info.summary, "Renamed");
    }

    #[tokio::test]
    async fn tag_sets_then_clears() {
        let tmp = tempfile::tempdir().expect("tmp");
        let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
        let dir = tmp.path().join("projects").join("-repo-a");
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        tokio::fs::write(
            dir.join(format!("{id}.jsonl")),
            b"{\"type\":\"user\",\"message\":{\"content\":\"q\"}}\n",
        )
        .await
        .expect("write");
        let a = SessionArchive::with_base(tmp.path());
        a.tag(id, Some("important"), None).await.expect("tag");
        assert_eq!(
            a.info(id, None)
                .await
                .expect("ok")
                .expect("some")
                .tag
                .as_deref(),
            Some("important")
        );
        a.tag(id, None, None).await.expect("clear tag"); // clear
        assert_eq!(a.info(id, None).await.expect("ok").expect("some").tag, None);
        assert!(matches!(
            a.tag(id, Some("  "), None).await,
            Err(SessionError::EmptyValue(_))
        ));
    }
}
