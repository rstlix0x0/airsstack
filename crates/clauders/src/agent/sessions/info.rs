//! Session metadata (`SessionInfo`) and its derivation from the head/tail
//! of a transcript file.

use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::agent::types::SessionId;

/// The head/tail window the binary reads for metadata: 64 KiB.
const WINDOW: usize = 65536;

/// A file's metadata window: first and last [`WINDOW`] bytes plus stat.
pub(crate) struct HeadTail {
    pub mtime_ms: i64,
    pub size: u64,
    pub head: String,
    pub tail: String,
}

/// Read the head/tail window of `path`. `None` when the file is absent or
/// empty (matching the binary's size-0 → null).
pub(crate) async fn read_head_tail(path: &Path) -> io::Result<Option<HeadTail>> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let meta = file.metadata().await?;
    let size = meta.len();
    if size == 0 {
        return Ok(None);
    }
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    let window = u64::try_from(WINDOW).unwrap_or(u64::MAX);
    let mut buf = vec![0u8; WINDOW];
    let n = file.read(&mut buf).await?;
    let head = String::from_utf8_lossy(&buf[..n]).into_owned();
    let tail = if size > window {
        file.seek(io::SeekFrom::Start(size - window)).await?;
        let n2 = file.read(&mut buf).await?;
        String::from_utf8_lossy(&buf[..n2]).into_owned()
    } else {
        head.clone()
    };
    Ok(Some(HeadTail {
        mtime_ms,
        size,
        head,
        tail,
    }))
}

/// Parse a UTC `YYYY-MM-DDTHH:MM:SS(.sss)?Z` timestamp to epoch milliseconds.
/// Dep-free civil-date conversion (Howard Hinnant's `days_from_civil`).
pub(crate) fn parse_iso_ms(ts: &str) -> Option<i64> {
    let (date, rest) = ts.split_once('T')?;
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;
    let time = rest.strip_suffix('Z').unwrap_or(rest);
    let (hms, frac) = match time.split_once('.') {
        Some((left, right)) => (left, right),
        None => (time, "0"),
    };
    let mut time_parts = hms.split(':');
    let hour: i64 = time_parts.next()?.parse().ok()?;
    let minute: i64 = time_parts.next()?.parse().ok()?;
    let second: i64 = time_parts.next()?.parse().ok()?;
    // milliseconds from up-to-3 fractional digits
    let mut millis = 0i64;
    for (index, ch) in frac.chars().take(3).enumerate() {
        let digit = i64::from(ch.to_digit(10)?);
        let exponent = u32::try_from(2 - index).unwrap_or(0);
        millis += digit * 10i64.pow(exponent);
    }
    // days_from_civil
    let civil_year = if month <= 2 { year - 1 } else { year };
    let era = if civil_year >= 0 {
        civil_year
    } else {
        civil_year - 399
    } / 400;
    let year_of_era = civil_year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let secs = days * 86400 + hour * 3600 + minute * 60 + second;
    Some(secs * 1000 + millis)
}

/// Metadata for one stored session, as returned by `list` and `info`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    /// The session's UUID.
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    /// Display title: custom title, else auto-summary, else first prompt.
    pub summary: String,
    /// Last-modified time in integer milliseconds since the epoch.
    #[serde(rename = "lastModified")]
    pub last_modified: i64,
    /// File size in bytes.
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    /// User-set custom title, when present.
    #[serde(
        rename = "customTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_title: Option<String>,
    /// First meaningful user prompt, when present.
    #[serde(
        rename = "firstPrompt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub first_prompt: Option<String>,
    /// Git branch at the end of the session, when present.
    #[serde(rename = "gitBranch", default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    /// Working directory for the session, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// User-set tag, when present and non-empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Creation time in integer milliseconds since the epoch, when derivable.
    #[serde(rename = "createdAt", default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

/// Build a [`SessionInfo`] from a head/tail window. Returns `None` when the
/// first entry is a sidechain, or when no summary source can be derived —
/// both cause the binary to drop the session from listings.
pub(crate) fn build_info(
    session_id: &str,
    ht: &HeadTail,
    project_path: Option<&str>,
) -> Option<SessionInfo> {
    let first_line = ht.head.split('\n').next().unwrap_or("");
    if first_line.contains("\"isSidechain\":true") || first_line.contains("\"isSidechain\": true") {
        return None;
    }
    let custom_title = last_string_field(&ht.tail, "customTitle")
        .or_else(|| last_string_field(&ht.head, "customTitle"))
        .or_else(|| last_string_field(&ht.tail, "aiTitle"))
        .or_else(|| last_string_field(&ht.head, "aiTitle"));
    let first_prompt = first_prompt(&ht.head);
    let created_at = first_string_field(&ht.head, "timestamp").and_then(|s| parse_iso_ms(&s));
    let summary = custom_title
        .clone()
        .or_else(|| last_string_field(&ht.tail, "lastPrompt"))
        .or_else(|| last_string_field(&ht.tail, "summary"))
        .or_else(|| first_prompt.clone())?;
    let git_branch = last_string_field(&ht.tail, "gitBranch")
        .or_else(|| first_string_field(&ht.head, "gitBranch"));
    let cwd = last_typed_field(&ht.tail, "relocated", "relocatedCwd")
        .or_else(|| first_string_field(&ht.head, "cwd"))
        .or_else(|| project_path.map(str::to_string));
    let tag = ht
        .tail
        .split('\n')
        .rev()
        .find(|l| l.contains("\"type\":\"tag\"") && l.contains("\"tag\":\""))
        .and_then(|l| last_string_field(l, "tag"))
        .filter(|t| !t.is_empty());
    Some(SessionInfo {
        session_id: SessionId::new(session_id),
        summary,
        last_modified: ht.mtime_ms,
        file_size: ht.size,
        custom_title,
        first_prompt,
        git_branch,
        cwd,
        tag,
        created_at,
    })
}

/// Unescape a raw JSON string body (the text between the quotes). Mirrors
/// the binary: a body with no backslash is returned as-is; otherwise it is
/// parsed as a JSON string.
fn unescape(raw: &str) -> String {
    if !raw.contains('\\') {
        return raw.to_string();
    }
    serde_json::from_str::<String>(&format!("\"{raw}\"")).unwrap_or_else(|_| raw.to_string())
}

/// Read the quoted string value starting just after `open` at byte `start`,
/// stopping at the first unescaped `"`. Returns the unescaped value and the
/// index of the opening pattern.
fn read_value_at(text: &str, value_start: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let mut c = value_start;
    while c < bytes.len() {
        match bytes[c] {
            b'\\' => c += 2,
            b'"' => return Some(unescape(&text[value_start..c])),
            _ => c += 1,
        }
    }
    None
}

/// First `"key":"…"` (or `"key": "…"`) string value in `text`.
fn first_string_field(text: &str, key: &str) -> Option<String> {
    for pat in [format!("\"{key}\":\""), format!("\"{key}\": \"")] {
        if let Some(a) = text.find(&pat) {
            return read_value_at(text, a + pat.len());
        }
    }
    None
}

/// Last `"key":"…"` (or `"key": "…"`) string value in `text` — the highest
/// starting position across both spacings wins.
fn last_string_field(text: &str, key: &str) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for pat in [format!("\"{key}\":\""), format!("\"{key}\": \"")] {
        let mut from = 0;
        while let Some(rel) = text[from..].find(&pat) {
            let a = from + rel;
            if let Some(v) = read_value_at(text, a + pat.len()) {
                if best.as_ref().is_none_or(|(pos, _)| a > *pos) {
                    best = Some((a, v));
                }
            }
            from = a + pat.len();
        }
    }
    best.map(|(_, v)| v)
}

/// The `key` string value from the last newline-delimited JSON object in
/// `text` whose `"type"` equals `ty`.
fn last_typed_field(text: &str, ty: &str, key: &str) -> Option<String> {
    let type_marker = format!("\"type\":\"{ty}\"");
    for line in text.split('\n').rev() {
        if line.contains(&type_marker) {
            if let Some(v) = last_string_field(line, key) {
                return Some(v);
            }
        }
    }
    None
}

/// The first meaningful user prompt in `head`: the first newline-delimited
/// user entry that is not a `tool_result`, meta, or compact-summary line. The
/// prompt text is the entry's `message.content` (a string, or the joined
/// text blocks of an array).
///
// The binary additionally special-cases slash-command messages via a
// command-fallback accumulator; that display nuance is not reproduced here
// (documented divergence — this field is display-only and the last fallback
// in the summary chain).
fn first_prompt(head: &str) -> Option<String> {
    for line in head.split('\n') {
        if !line.contains("\"type\":\"user\"") && !line.contains("\"type\": \"user\"") {
            continue;
        }
        if line.contains("\"tool_result\"")
            || line.contains("\"isMeta\":true")
            || line.contains("\"isMeta\": true")
            || line.contains("\"isCompactSummary\":true")
            || line.contains("\"isCompactSummary\": true")
        {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let content = &entry["message"]["content"];
        if let Some(s) = content.as_str() {
            return Some(s.to_string());
        }
        if let Some(arr) = content.as_array() {
            let text: String = arr
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("");
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

#[cfg(test)]
mod extractor_tests {
    use super::*;

    #[test]
    fn last_string_field_takes_the_latest_occurrence() {
        let t = "{\"customTitle\":\"old\"}\n{\"customTitle\":\"new\"}";
        assert_eq!(last_string_field(t, "customTitle").as_deref(), Some("new"));
    }

    #[test]
    fn first_string_field_takes_the_earliest() {
        let t = "{\"timestamp\":\"2026-07-23T09:37:06.000Z\"}\n{\"timestamp\":\"2026-07-23T10:00:00.000Z\"}";
        assert_eq!(
            first_string_field(t, "timestamp").as_deref(),
            Some("2026-07-23T09:37:06.000Z")
        );
    }

    #[test]
    fn last_typed_field_scans_from_the_end_by_type() {
        let t = "{\"type\":\"user\",\"x\":1}\n{\"type\":\"relocated\",\"relocatedCwd\":\"/moved/repo\"}";
        assert_eq!(
            last_typed_field(t, "relocated", "relocatedCwd").as_deref(),
            Some("/moved/repo")
        );
    }

    #[test]
    fn first_prompt_reads_array_text_blocks() {
        let head =
            r#"{"type":"user","message":{"content":[{"type":"text","text":"hello world"}]}}"#;
        assert_eq!(first_prompt(head).as_deref(), Some("hello world"));
    }

    #[test]
    fn first_prompt_skips_tool_result_and_meta() {
        let head = concat!(
            r#"{"type":"user","isMeta":true,"message":{"content":"x"}}"#,
            "\n",
            r#"{"type":"user","message":{"content":[{"type":"tool_result"}]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"real prompt"}}"#
        );
        assert_eq!(first_prompt(head).as_deref(), Some("real prompt"));
    }
}

#[cfg(test)]
mod reader_tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::*;

    #[tokio::test]
    async fn reads_whole_small_file_as_head_and_tail() {
        let tmp = tempfile::tempdir().expect("tmp");
        let p = tmp.path().join("s.jsonl");
        tokio::fs::write(&p, b"line1\nline2\n")
            .await
            .expect("write");
        let ht = read_head_tail(&p).await.expect("io").expect("some");
        assert_eq!(ht.head, "line1\nline2\n");
        assert_eq!(ht.tail, "line1\nline2\n");
        assert_eq!(ht.size, 12);
    }

    #[tokio::test]
    async fn empty_file_is_none() {
        let tmp = tempfile::tempdir().expect("tmp");
        let p = tmp.path().join("e.jsonl");
        tokio::fs::write(&p, b"").await.expect("write");
        assert!(read_head_tail(&p).await.expect("io").is_none());
    }

    #[test]
    fn parses_utc_iso_to_epoch_ms() {
        // 2026-07-23T09:37:06.000Z — verify against a known epoch.
        let ms = parse_iso_ms("2026-07-23T09:37:06.000Z").expect("parse");
        assert_eq!(ms, 1_784_799_426_000);
    }

    #[test]
    fn rejects_malformed_timestamp() {
        assert!(parse_iso_ms("not-a-date").is_none());
    }
}

#[cfg(test)]
mod build_tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::*;

    fn ht(head: &str, tail: &str) -> HeadTail {
        HeadTail {
            mtime_ms: 42,
            size: 7,
            head: head.to_string(),
            tail: tail.to_string(),
        }
    }

    #[test]
    fn custom_title_wins_the_summary_and_is_reported() {
        // customTitle beats lastPrompt in the summary chain.
        let head = r#"{"type":"user","timestamp":"2026-07-23T09:37:06.000Z","cwd":"/repo","message":{"content":"hi there"}}"#;
        let tail = concat!(
            r#"{"type":"lastPrompt","lastPrompt":"later prompt"}"#,
            "\n",
            r#"{"type":"custom-title","customTitle":"My Session"}"#
        );
        let info = build_info(
            "f28ced56-9bd4-41f8-a37d-2a496c7d0e35",
            &ht(head, tail),
            Some("/proj"),
        )
        .expect("info");
        assert_eq!(info.summary, "My Session");
        assert_eq!(info.custom_title.as_deref(), Some("My Session"));
        assert_eq!(info.first_prompt.as_deref(), Some("hi there"));
        assert_eq!(info.created_at, Some(1_784_799_426_000));
        assert_eq!(info.cwd.as_deref(), Some("/repo"));
        assert_eq!(info.last_modified, 42);
    }

    #[test]
    fn last_tag_wins_and_empty_tag_is_omitted() {
        let head = r#"{"type":"user","message":{"content":"q"}}"#;
        let tail_set = concat!(
            r#"{"type":"tag","tag":"first"}"#,
            "\n",
            r#"{"type":"tag","tag":"second"}"#
        );
        let info = build_info(
            "f28ced56-9bd4-41f8-a37d-2a496c7d0e35",
            &ht(head, tail_set),
            None,
        )
        .expect("info");
        assert_eq!(info.tag.as_deref(), Some("second"));

        let tail_cleared = format!("{tail_set}\n{}", r#"{"type":"tag","tag":""}"#);
        let cleared = build_info(
            "f28ced56-9bd4-41f8-a37d-2a496c7d0e35",
            &ht(head, &tail_cleared),
            None,
        )
        .expect("info");
        assert_eq!(cleared.tag, None, "empty tag record clears the tag");
    }

    #[test]
    fn sidechain_first_entry_is_skipped() {
        let head = r#"{"type":"user","isSidechain":true,"message":{"content":"x"}}"#;
        assert!(
            build_info(
                "f28ced56-9bd4-41f8-a37d-2a496c7d0e35",
                &ht(head, head),
                None
            )
            .is_none()
        );
    }

    #[test]
    fn no_summary_source_drops_the_session() {
        let head = r#"{"type":"system","subtype":"init"}"#;
        assert!(
            build_info(
                "f28ced56-9bd4-41f8-a37d-2a496c7d0e35",
                &ht(head, head),
                None
            )
            .is_none()
        );
    }
}
