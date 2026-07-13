//! Filesystem-backed conversation-history store for the native runtime.
//!
//! One JSON file per session at `<root>/<session_id>.json`. Writes are
//! atomic (temp file + rename). A missing file reads back as an empty
//! history; a corrupt file is an error.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::agent::error::AgentError;
use crate::agent::types::SessionId;
use crate::messages::request::InputMessage;

/// The default session-store root when `Options::session_dir` is unset:
/// `$HOME/.clauders/sessions`, falling back to the system temp dir when
/// `$HOME` is not set.
pub(super) fn default_root() -> PathBuf {
    std::env::var_os("HOME").map_or_else(
        || std::env::temp_dir().join("clauders").join("sessions"),
        |home| PathBuf::from(home).join(".clauders").join("sessions"),
    )
}

/// The load source and write target session ids for one runtime.
///
/// They differ only for a forked session's first run; the runtime collapses
/// `load` onto `write` after the first successful persist (in
/// [`SessionSink::save`]) so the fork continues in place thereafter.
#[derive(Clone, Debug)]
pub(super) struct SessionIds {
    pub(super) load: SessionId,
    pub(super) write: SessionId,
}

impl SessionIds {
    pub(super) const fn new(load: SessionId, write: SessionId) -> Self {
        Self { load, write }
    }
}

/// A filesystem store of conversation histories.
#[derive(Clone)]
pub(super) struct SessionStore {
    root: PathBuf,
}

impl SessionStore {
    pub(super) const fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, id: &SessionId) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    /// Load the persisted turns for `id`. A missing file yields an empty
    /// history; a present-but-corrupt file is an error.
    pub(super) fn load(&self, id: &SessionId) -> Result<Vec<InputMessage>, AgentError> {
        match std::fs::read(self.path(id)) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| AgentError::SessionStore {
                detail: format!("failed to decode session `{id}`: {error}"),
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(AgentError::SessionStore {
                detail: format!("failed to read session `{id}`: {error}"),
            }),
        }
    }

    /// Persist `turns` for `id` atomically: write a temp file, then rename
    /// it over the destination so a crash mid-write never corrupts an
    /// existing session.
    pub(super) fn save(&self, id: &SessionId, turns: &[InputMessage]) -> Result<(), AgentError> {
        std::fs::create_dir_all(&self.root).map_err(|error| AgentError::SessionStore {
            detail: format!(
                "failed to create session store `{}`: {error}",
                self.root.display()
            ),
        })?;
        let json = serde_json::to_vec(turns).map_err(|error| AgentError::SessionStore {
            detail: format!("failed to encode session `{id}`: {error}"),
        })?;
        let tmp = self.root.join(format!("{id}.json.tmp"));
        std::fs::write(&tmp, &json).map_err(|error| AgentError::SessionStore {
            detail: format!("failed to write session `{id}`: {error}"),
        })?;
        std::fs::rename(&tmp, self.path(id)).map_err(|error| AgentError::SessionStore {
            detail: format!("failed to commit session `{id}`: {error}"),
        })
    }

    /// Whether a session file exists for `id`.
    pub(super) fn exists(&self, id: &SessionId) -> bool {
        self.path(id).exists()
    }

    /// The id of the most recently modified session, if any.
    pub(super) fn most_recent(&self) -> Option<SessionId> {
        let entries = std::fs::read_dir(&self.root).ok()?;
        entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|entry| {
                let modified = entry.metadata().ok()?.modified().ok()?;
                let stem = entry.path().file_stem()?.to_str()?.to_string();
                Some((modified, SessionId::new(stem)))
            })
            .max_by_key(|(modified, _)| *modified)
            .map(|(_, id)| id)
    }

    /// Enumerate the ids of all stored sessions (read-only discovery). A
    /// missing store directory is an empty list, not an error.
    pub(super) fn list(&self) -> Result<Vec<SessionId>, AgentError> {
        let entries = match std::fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(AgentError::SessionStore {
                    detail: format!(
                        "failed to list session store `{}`: {error}",
                        self.root.display()
                    ),
                });
            }
        };
        let mut ids = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    ids.push(SessionId::new(stem));
                }
            }
        }
        Ok(ids)
    }
}

/// A bound write target: a store, the session id to persist to, and a shared
/// handle to the runtime's session ids so a successful save can collapse the
/// load pointer onto the write target.
#[derive(Clone)]
pub(super) struct SessionSink {
    store: SessionStore,
    write: SessionId,
    session: Arc<Mutex<SessionIds>>,
}

impl SessionSink {
    pub(super) const fn new(
        store: SessionStore,
        write: SessionId,
        session: Arc<Mutex<SessionIds>>,
    ) -> Self {
        Self {
            store,
            write,
            session,
        }
    }

    /// Persist `turns`, then — only on a successful save — collapse the shared
    /// load pointer onto this write target so a subsequent run continues in
    /// place. A failed save leaves the load pointer untouched (re-seedable).
    pub(super) fn save(&self, turns: &[InputMessage]) -> Result<(), AgentError> {
        self.store.save(&self.write, turns)?;
        {
            let mut ids = self
                .session
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            ids.load = self.write.clone();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "tests assert known-valid fixtures")]

    use super::SessionStore;
    use crate::agent::types::SessionId;
    use crate::messages::request::{InputMessage, MessageContent, Role};

    fn turn(text: &str) -> InputMessage {
        InputMessage {
            role: Role::User,
            content: MessageContent::Text(text.to_string()),
        }
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path().to_path_buf());
        let id = SessionId::new("sess_rt");
        let turns = vec![turn("hi")];
        store.save(&id, &turns).expect("save");
        assert_eq!(store.load(&id).expect("load"), turns);
    }

    #[test]
    fn missing_file_loads_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path().to_path_buf());
        assert!(
            store
                .load(&SessionId::new("nope"))
                .expect("load")
                .is_empty()
        );
    }

    #[test]
    fn corrupt_file_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path().to_path_buf());
        std::fs::create_dir_all(dir.path()).expect("mkdir");
        std::fs::write(dir.path().join("sess_bad.json"), b"not json").expect("write");
        assert!(store.load(&SessionId::new("sess_bad")).is_err());
    }

    #[test]
    fn save_leaves_no_tmp_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path().to_path_buf());
        store.save(&SessionId::new("sess_t"), &[]).expect("save");
        let has_tmp = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"));
        assert!(!has_tmp, "no .tmp file should remain after save");
    }

    #[test]
    fn exists_reflects_saved_sessions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path().to_path_buf());
        let id = SessionId::new("sess_e");
        assert!(!store.exists(&id));
        store.save(&id, &[]).expect("save");
        assert!(store.exists(&id));
    }

    #[test]
    fn most_recent_picks_newest_by_mtime() {
        use std::time::{Duration, SystemTime};
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path().to_path_buf());
        store.save(&SessionId::new("old"), &[]).expect("save old");
        store.save(&SessionId::new("new"), &[]).expect("save new");
        // Force deterministic ordering regardless of filesystem mtime resolution.
        let base = SystemTime::now();
        std::fs::File::options()
            .write(true)
            .open(dir.path().join("old.json"))
            .expect("open old")
            .set_modified(base - Duration::from_secs(10))
            .expect("set old mtime");
        std::fs::File::options()
            .write(true)
            .open(dir.path().join("new.json"))
            .expect("open new")
            .set_modified(base)
            .expect("set new mtime");
        assert_eq!(store.most_recent().expect("most_recent").as_str(), "new");
    }
}
