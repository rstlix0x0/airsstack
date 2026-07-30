//! Config-root resolution, session-id validation, cwd encoding, and the
//! session-file finder — all matching the binary's on-disk layout.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::error::SessionError;

/// Upper bound on the `git worktree list` invocation in [`worktree_paths`],
/// matching the binary's bound on the same call.
const WORKTREE_LIST_TIMEOUT: Duration = Duration::from_secs(5);

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

/// A located session file: its path, the cwd it was scoped to (when the
/// lookup supplied one), and its byte size.
pub(crate) struct FoundSession {
    pub file_path: PathBuf,
    pub project_path: Option<String>,
    #[expect(
        dead_code,
        reason = "will back per-directory session entries once list/dir enumeration is added"
    )]
    pub file_size: u64,
}

/// Try one directory for `<id>.jsonl` with size > 0.
async fn probe(dir: &Path, file_name: &str, project_path: Option<&str>) -> Option<FoundSession> {
    let file_path = dir.join(file_name);
    match tokio::fs::metadata(&file_path).await {
        Ok(m) if m.is_file() && m.len() > 0 => Some(FoundSession {
            file_path,
            project_path: project_path.map(str::to_string),
            file_size: m.len(),
        }),
        _ => None,
    }
}

/// The git worktrees linked to `cwd`, via `git worktree list --porcelain`.
/// Returns an empty vector when git is absent, `cwd` is not a work tree, the
/// command errors, or it exceeds [`WORKTREE_LIST_TIMEOUT`] — never an error
/// (matching the binary's `catch → []`, extended to a hung child process).
pub(crate) async fn worktree_paths(cwd: &str) -> Vec<String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.args([
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=",
        "worktree",
        "list",
        "--porcelain",
    ])
    .current_dir(cwd)
    .kill_on_drop(true);

    let Ok(Ok(out)) = tokio::time::timeout(WORKTREE_LIST_TIMEOUT, cmd.output()).await else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.strip_prefix("worktree "))
        .map(str::to_string)
        .collect()
}

/// Candidate directories for `cwd` **and** its linked worktrees, each tagged
/// with the worktree path it came from (for `project_path`).
pub(crate) async fn candidate_dirs_with_worktrees(
    root: &Path,
    cwd: &str,
) -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = candidate_dirs(root, cwd)
        .await
        .into_iter()
        .map(|d| (d, cwd.to_string()))
        .collect();
    for wt in worktree_paths(cwd).await {
        if wt == cwd {
            continue;
        }
        for d in candidate_dirs(root, &wt).await {
            out.push((d, wt.clone()));
        }
    }
    out
}

/// Append `text` to `path` iff it exists and is non-empty. Returns whether
/// the append happened. A genuine I/O error (not "absent") propagates.
pub(crate) async fn append_to_file(path: &Path, text: &str) -> std::io::Result<bool> {
    use tokio::io::AsyncWriteExt;
    let mut file = match tokio::fs::OpenOptions::new().append(true).open(path).await {
        Ok(f) => f,
        Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound) => return Ok(false),
        // ENOTDIR and other "path does not resolve" cases also mean "not here".
        Err(e) if e.raw_os_error() == Some(20) => return Ok(false),
        Err(e) => return Err(e),
    };
    let meta = file.metadata().await?;
    if meta.len() == 0 {
        return Ok(false);
    }
    file.write_all(text.as_bytes()).await?;
    Ok(true)
}

/// Append `text` to session `session_id`'s file, locating it the way the
/// binary does. With `dir`, search that cwd's candidate + worktree
/// directories; otherwise scan every project subdirectory. Errors with
/// [`SessionError::SessionNotFound`] when no session file matches.
pub(crate) async fn append_session_record(
    root: &Path,
    session_id: &str,
    dir: Option<&str>,
    text: &str,
) -> Result<(), SessionError> {
    let file_name = format!("{session_id}.jsonl");
    if let Some(cwd) = dir {
        for (candidate, _wt) in candidate_dirs_with_worktrees(root, cwd).await {
            if append_to_file(&candidate.join(&file_name), text).await? {
                return Ok(());
            }
        }
        return Err(SessionError::SessionNotFound {
            session_id: session_id.to_string(),
            location: format!(" in project directory for {cwd}"),
        });
    }
    let Ok(mut rd) = tokio::fs::read_dir(root).await else {
        return Err(SessionError::SessionNotFound {
            session_id: session_id.to_string(),
            location: " (no projects directory)".to_string(),
        });
    };
    while let Ok(Some(sub)) = rd.next_entry().await {
        if sub.file_type().await.map(|t| t.is_dir()).unwrap_or(false)
            && append_to_file(&sub.path().join(&file_name), text).await?
        {
            return Ok(());
        }
    }
    Err(SessionError::SessionNotFound {
        session_id: session_id.to_string(),
        location: " in any project directory".to_string(),
    })
}

/// Locate a session's `.jsonl`. With `dir`, search that cwd's candidate
/// project directories (and any linked git worktrees); otherwise scan every
/// subdirectory of `root`. Only a non-empty file counts.
pub(crate) async fn find_session_file(
    root: &Path,
    session_id: &str,
    dir: Option<&str>,
) -> Option<FoundSession> {
    let file_name = format!("{session_id}.jsonl");
    if let Some(cwd) = dir {
        for candidate in candidate_dirs(root, cwd).await {
            if let Some(found) = probe(&candidate, &file_name, Some(cwd)).await {
                return Some(found);
            }
        }
        for (candidate, wt) in candidate_dirs_with_worktrees(root, cwd).await {
            if let Some(found) = probe(&candidate, &file_name, Some(&wt)).await {
                return Some(found);
            }
        }
        return None;
    }
    let mut rd = tokio::fs::read_dir(root).await.ok()?;
    while let Ok(Some(entry)) = rd.next_entry().await {
        if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(found) = probe(&entry.path(), &file_name, None).await {
                return Some(found);
            }
        }
    }
    None
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

    #[tokio::test]
    async fn finds_a_session_by_scanning_all_project_dirs() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("projects");
        let dir = root.join("-repo-a");
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
        tokio::fs::write(dir.join(format!("{id}.jsonl")), b"x\n")
            .await
            .expect("write");
        let found = find_session_file(&root, id, None).await.expect("found");
        assert_eq!(found.file_path, dir.join(format!("{id}.jsonl")));
        assert!(found.project_path.is_none());
    }

    #[tokio::test]
    async fn finds_a_session_scoped_to_a_dir() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("projects");
        let cwd = "/repo/b";
        let dir = root.join(encode_cwd(cwd));
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
        tokio::fs::write(dir.join(format!("{id}.jsonl")), b"x\n")
            .await
            .expect("write");
        let found = find_session_file(&root, id, Some(cwd))
            .await
            .expect("found");
        assert_eq!(found.file_path, dir.join(format!("{id}.jsonl")));
        assert_eq!(found.project_path.as_deref(), Some(cwd));
    }

    #[tokio::test]
    async fn empty_session_file_is_not_found() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("projects");
        let dir = root.join("-repo-c");
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
        tokio::fs::write(dir.join(format!("{id}.jsonl")), b"")
            .await
            .expect("write");
        assert!(find_session_file(&root, id, None).await.is_none());
    }

    #[tokio::test]
    async fn worktree_dirs_returns_empty_outside_git() {
        // Outside a git tree (or with git absent) enumeration yields nothing,
        // never an error.
        let tmp = tempfile::tempdir().expect("tmp");
        assert!(
            worktree_paths(&tmp.path().to_string_lossy())
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn append_returns_false_for_absent_or_empty_file() {
        let tmp = tempfile::tempdir().expect("tmp");
        let missing = tmp.path().join("nope.jsonl");
        assert!(!append_to_file(&missing, "x\n").await.expect("ok"));
        let empty = tmp.path().join("empty.jsonl");
        tokio::fs::write(&empty, b"").await.expect("write");
        assert!(!append_to_file(&empty, "x\n").await.expect("ok"));
    }

    #[tokio::test]
    async fn append_writes_to_a_nonempty_file() {
        let tmp = tempfile::tempdir().expect("tmp");
        let p = tmp.path().join("s.jsonl");
        tokio::fs::write(&p, b"first\n").await.expect("write");
        assert!(append_to_file(&p, "second\n").await.expect("ok"));
        let got = tokio::fs::read_to_string(&p).await.expect("read");
        assert_eq!(got, "first\nsecond\n");
    }

    #[tokio::test]
    async fn appends_to_a_session_found_by_scanning() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("projects");
        let dir = root.join("-repo-a");
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
        let p = dir.join(format!("{id}.jsonl"));
        tokio::fs::write(&p, b"first\n").await.expect("write");
        append_session_record(&root, id, None, "rec\n")
            .await
            .expect("appended");
        assert_eq!(
            tokio::fs::read_to_string(&p).await.expect("read"),
            "first\nrec\n"
        );
    }

    #[tokio::test]
    async fn errors_when_no_session_matches() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("projects");
        tokio::fs::create_dir_all(&root).await.expect("mkdir");
        let err =
            append_session_record(&root, "f28ced56-9bd4-41f8-a37d-2a496c7d0e35", None, "rec\n")
                .await
                .expect_err("not found");
        assert!(matches!(err, SessionError::SessionNotFound { .. }));
    }

    #[tokio::test]
    async fn worktree_dirs_returns_promptly_well_under_the_timeout_bound() {
        // A real 5s hang is impractical to trigger deterministically in a
        // unit test. This instead proves the fast, non-hanging path stays
        // far under WORKTREE_LIST_TIMEOUT, so the wrapper isn't silently
        // stretching every call out to the bound; the timeout-elapses branch
        // (`Vec::new()` on `Err` from `tokio::time::timeout`) is exercised by
        // code inspection: it shares the exact same catch-arm as the
        // git-absent/non-worktree/non-zero-exit cases covered above.
        let tmp = tempfile::tempdir().expect("tmp");
        let start = std::time::Instant::now();
        let got = worktree_paths(&tmp.path().to_string_lossy()).await;
        let elapsed = start.elapsed();
        assert!(got.is_empty());
        assert!(
            elapsed < WORKTREE_LIST_TIMEOUT / 2,
            "expected a fast return well under the 5s timeout bound, took {elapsed:?}"
        );
    }
}
