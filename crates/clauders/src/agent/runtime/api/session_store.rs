//! Filesystem-backed conversation-history store for the native runtime.
//!
//! One JSON file per session at `<root>/<session_id>.json`. Writes are
//! atomic (temp file + rename). A missing file reads back as an empty
//! history; a corrupt file is an error.

use std::path::PathBuf;

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
/// They differ only for a forked session's first run; `run` collapses
/// `load` onto `write` after the first persist so the fork continues in
/// place thereafter.
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
}

/// A bound write target: a store plus the session id to persist to.
#[derive(Clone)]
pub(super) struct SessionSink {
    store: SessionStore,
    write: SessionId,
}

impl SessionSink {
    pub(super) const fn new(store: SessionStore, write: SessionId) -> Self {
        Self { store, write }
    }

    pub(super) fn save(&self, turns: &[InputMessage]) -> Result<(), AgentError> {
        self.store.save(&self.write, turns)
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
}
