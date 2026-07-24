//! Session metadata (`SessionInfo`) and its derivation from the head/tail
//! of a transcript file.

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
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "will back SessionInfo::created_at derivation once that is wired in"
    )
)]
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
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "will back SessionInfo::relocated_cwd derivation once that is wired in"
    )
)]
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
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "will back SessionInfo::first_prompt derivation once that is wired in"
    )
)]
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
