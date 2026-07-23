//! Response payloads for task, turn, and workspace control operations.

use serde::{Deserialize, Serialize};

/// Result of `read_file`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadFileResult {
    /// The file contents (utf-8, or base64 when `encoding` says so).
    pub contents: String,
    /// The absolute path read.
    #[serde(rename = "absPath")]
    pub abs_path: String,
    /// Whether the contents were truncated at `max_bytes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    /// The encoding of `contents` when not utf-8 (e.g. `base64`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

/// Result of `rewind_files`.
///
/// The response is built by the `rewindFiles` handler and matches its zod
/// schema verbatim
/// (`{canRewind, error?, filesChanged?, insertions?, deletions?, skippedLinks?}`):
/// refusal returns only `{canRewind:false, error}`; a dry run returns
/// `{canRewind:true, filesChanged, insertions, deletions}`; a real rewind
/// returns `{canRewind:true, skippedLinks}`. No single response carries every
/// field, so all but `can_rewind` are optional.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewindFilesResult {
    /// Whether a rewind is possible / was performed.
    #[serde(rename = "canRewind")]
    pub can_rewind: bool,
    /// Refusal or failure detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Paths changed by the rewind (present on a dry run).
    #[serde(
        rename = "filesChanged",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub files_changed: Option<Vec<String>>,
    /// Lines inserted (present on a dry run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insertions: Option<u64>,
    /// Lines deleted (present on a dry run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletions: Option<u64>,
    /// Count of symlinked files skipped by an actual (non-dry-run) rewind.
    #[serde(
        rename = "skippedLinks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub skipped_links: Option<u64>,
}

/// Result of `background_tasks`.
///
/// The server dispatch for `subtype:"background_tasks"` builds
/// `{backgrounded: <bool>}` when the request named a single `tool_use_id`
/// (the backgrounding outcome for that task), and `{}` — no `backgrounded`
/// key at all — when it backgrounded every foreground task. The SDK's own
/// client reads this field as `response.backgrounded ?? true`, i.e. absence
/// means "succeeded, backgrounding everything". This is a boolean flag, not
/// a list of task ids or a count.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundTasksResult {
    /// Whether the targeted task was backgrounded, when a single
    /// `tool_use_id` was requested. Absent when all foreground tasks were
    /// backgrounded (treat absence as success, matching the SDK client).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backgrounded: Option<bool>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]
    use super::{BackgroundTasksResult, ReadFileResult, RewindFilesResult};

    #[test]
    fn read_file_result_binds_verbatim_shape() {
        // [binary v2.1.216], verbatim response builder:
        // { contents, absPath, ...truncated?, ...encoding? }
        let json = r#"{"contents":"hi","absPath":"/tmp/x","truncated":true,"encoding":"base64"}"#;
        let r: ReadFileResult = serde_json::from_str(json).expect("deserialize");
        assert_eq!(r.contents, "hi");
        assert_eq!(r.abs_path, "/tmp/x");
        assert_eq!(r.truncated, Some(true));
        assert_eq!(r.encoding.as_deref(), Some("base64"));
    }

    #[test]
    fn read_file_result_omits_optional_fields() {
        let json = r#"{"contents":"hi","absPath":"/tmp/x"}"#;
        let r: ReadFileResult = serde_json::from_str(json).expect("deserialize");
        assert!(r.truncated.is_none());
        assert!(r.encoding.is_none());
    }

    #[test]
    fn rewind_files_result_binds_can_rewind_and_error() {
        // [binary v2.1.216]: rewindFiles returns { canRewind:false, error:"..." }
        // on refusal (e.g. file history disabled, or no checkpoint found).
        let json = r#"{"canRewind":false,"error":"File rewinding is not enabled."}"#;
        let r: RewindFilesResult = serde_json::from_str(json).expect("deserialize");
        assert!(!r.can_rewind);
        assert_eq!(r.error.as_deref(), Some("File rewinding is not enabled."));
        assert!(r.files_changed.is_none());
        assert!(r.skipped_links.is_none());
    }

    #[test]
    fn rewind_files_result_binds_dry_run_shape() {
        // [probe v2.1.216]: the `pe?.dryRun` branch returns
        // {canRewind:true, filesChanged, insertions, deletions} — no
        // skippedLinks in this branch.
        let json =
            r#"{"canRewind":true,"filesChanged":["a.rs","b.rs"],"insertions":3,"deletions":1}"#;
        let r: RewindFilesResult = serde_json::from_str(json).expect("deserialize");
        assert!(r.can_rewind);
        assert_eq!(
            r.files_changed,
            Some(vec!["a.rs".to_string(), "b.rs".to_string()])
        );
        assert_eq!(r.insertions, Some(3));
        assert_eq!(r.deletions, Some(1));
        assert!(r.skipped_links.is_none());
    }

    #[test]
    fn rewind_files_result_binds_real_rewind_shape() {
        // [probe v2.1.216]: a real (non-dry-run) rewind returns
        // {canRewind:true, skippedLinks} — no filesChanged/insertions/deletions.
        let json = r#"{"canRewind":true,"skippedLinks":2}"#;
        let r: RewindFilesResult = serde_json::from_str(json).expect("deserialize");
        assert!(r.can_rewind);
        assert_eq!(r.skipped_links, Some(2));
        assert!(r.files_changed.is_none());
    }

    #[test]
    fn background_tasks_result_binds_backgrounded_bool_for_a_single_task() {
        // [probe v2.1.216]: the server dispatch builds {backgrounded: <bool>}
        // when the request named a single tool_use_id.
        let json = r#"{"backgrounded":true}"#;
        let r: BackgroundTasksResult = serde_json::from_str(json).expect("deserialize");
        assert_eq!(r.backgrounded, Some(true));
    }

    #[test]
    fn background_tasks_result_omits_backgrounded_when_all_tasks_targeted() {
        // [probe v2.1.216]: backgrounding every foreground task (no
        // tool_use_id in the request) returns `{}` on the wire.
        let json = r"{}";
        let r: BackgroundTasksResult = serde_json::from_str(json).expect("deserialize");
        assert!(r.backgrounded.is_none());
    }
}
