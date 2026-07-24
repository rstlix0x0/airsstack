//! Config-root resolution, session-id validation, cwd encoding, and the
//! session-file finder — all matching the binary's on-disk layout.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::error::SessionError;

/// Resolve the config root: `CLAUDE_CONFIG_DIR` when set (even if empty),
/// otherwise `$HOME/.claude`.
pub(crate) fn resolve_config_root() -> Result<PathBuf, SessionError> {
    resolve_config_root_from(
        std::env::var_os("CLAUDE_CONFIG_DIR"),
        std::env::var_os("HOME"),
    )
}

/// Environment-injected core of [`resolve_config_root`], for deterministic
/// testing without mutating the real process environment.
fn resolve_config_root_from(
    config_dir: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, SessionError> {
    if let Some(dir) = config_dir {
        return Ok(PathBuf::from(dir));
    }
    let home = home.ok_or(SessionError::NoConfigRoot)?;
    Ok(Path::new(&home).join(".claude"))
}

/// The directory that holds one project's session files:
/// `<base>/projects`.
pub(crate) fn projects_root(base: &Path) -> PathBuf {
    base.join("projects")
}

/// Whether `s` is a canonical `8-4-4-4-12` hex UUID (case-insensitive).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "will gate rename/tag session-id validation once those operations are added"
    )
)]
pub(crate) fn is_session_id(s: &str) -> bool {
    let groups = [8usize, 4, 4, 4, 12];
    let mut parts = s.split('-');
    for &len in &groups {
        match parts.next() {
            Some(part) if part.len() == len && part.bytes().all(|b| b.is_ascii_hexdigit()) => {}
            _ => return false,
        }
    }
    parts.next().is_none()
}

/// Maximum encoded directory-name length before the binary truncates+hashes.
const MAX_ENCODED_LEN: usize = 200;

/// Encode a cwd into a project directory name: every non-alphanumeric byte
/// becomes `-`. (The binary appends a hash past 200 chars; see
/// [`candidate_dirs`] for how the over-long case is handled.)
pub(crate) fn encode_cwd(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The project directories that may hold sessions for `cwd`, under `root`
/// (`<base>/projects`). For an encoding within the length cap this is the
/// single exact directory (when it exists); for an over-long encoding it is
/// every directory whose name starts with the 200-char prefix.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "will back the session finder used by list/info once those operations are added"
    )
)]
pub(crate) async fn candidate_dirs(root: &Path, cwd: &str) -> Vec<PathBuf> {
    let encoded = encode_cwd(cwd);
    if encoded.len() <= MAX_ENCODED_LEN {
        let exact = root.join(&encoded);
        return match tokio::fs::metadata(&exact).await {
            Ok(m) if m.is_dir() => vec![exact],
            _ => Vec::new(),
        };
    }
    let prefix = format!("{}-", &encoded[..MAX_ENCODED_LEN]);
    let mut out = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                out.push(entry.path());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::*;

    #[test]
    fn config_root_prefers_env_override() {
        let got = resolve_config_root_from(Some(OsString::from("/tmp/cfg-override")), None)
            .expect("resolves");
        assert_eq!(got, PathBuf::from("/tmp/cfg-override"));
    }

    #[test]
    fn config_root_falls_back_to_home_dot_claude() {
        let got =
            resolve_config_root_from(None, Some(OsString::from("/home/x"))).expect("resolves");
        assert_eq!(got, PathBuf::from("/home/x/.claude"));
    }

    #[test]
    fn config_root_errors_without_env_or_home() {
        assert!(matches!(
            resolve_config_root_from(None, None),
            Err(SessionError::NoConfigRoot)
        ));
    }

    #[test]
    fn projects_root_appends_projects() {
        assert_eq!(
            projects_root(Path::new("/x/.claude")),
            PathBuf::from("/x/.claude/projects")
        );
    }

    #[test]
    fn accepts_a_canonical_uuid() {
        assert!(is_session_id("f28ced56-9bd4-41f8-a37d-2a496c7d0e35"));
        assert!(is_session_id("F28CED56-9BD4-41F8-A37D-2A496C7D0E35"));
    }

    #[test]
    fn rejects_non_uuids() {
        assert!(!is_session_id("not-a-uuid"));
        assert!(!is_session_id("f28ced56-9bd4-41f8-a37d-2a496c7d0e3")); // 11 in last group
        assert!(!is_session_id("f28ced56-9bd4-41f8-a37d-2a496c7d0e35-extra"));
        assert!(!is_session_id("g28ced56-9bd4-41f8-a37d-2a496c7d0e35")); // non-hex
    }

    #[test]
    fn encodes_cwd_replacing_non_alnum_with_dash() {
        assert_eq!(encode_cwd("/Users/x/proj"), "-Users-x-proj");
        assert_eq!(encode_cwd("/a.b/c_d"), "-a-b-c-d");
    }

    #[tokio::test]
    async fn candidate_dirs_returns_the_exact_encoded_dir_when_present() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let root = tmp.path().join("projects");
        let cwd = "/repo/one";
        let encoded = encode_cwd(cwd); // "-repo-one"
        let dir = root.join(&encoded);
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        let got = candidate_dirs(&root, cwd).await;
        assert_eq!(got, vec![dir]);
    }

    #[tokio::test]
    async fn candidate_dirs_is_empty_when_absent() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let root = tmp.path().join("projects");
        tokio::fs::create_dir_all(&root).await.expect("mkdir");
        assert!(candidate_dirs(&root, "/nope").await.is_empty());
    }
}
