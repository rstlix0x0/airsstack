//! Reading a session's transcript into typed messages: parse, dedup,
//! filter, and map to `SessionMessage`.
//!
//! This deliberately returns the flat transcript (with `parent_uuid`
//! preserved), not the official SDK's active-thread reconstruction — a
//! documented divergence chosen for a transparent, lossless Rust contract.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::agent::message::Message;
use crate::agent::types::SessionId;

/// One transcript message: the typed payload plus the on-disk envelope,
/// including `parent_uuid` so a caller can reconstruct conversation
/// branches if it wishes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMessage {
    /// The message's own uuid on disk.
    pub uuid: String,
    /// The parent message's uuid — the branch structure, unresolved.
    pub parent_uuid: Option<String>,
    /// The session id recorded on the entry (may differ from the queried id).
    pub session_id: SessionId,
    /// The typed payload, reusing the streaming `Message` variants
    /// (`User`/`Assistant`/`System` are the only kinds this path yields).
    pub payload: SessionPayload,
    /// The entry's raw timestamp string.
    pub timestamp: String,
}

/// The typed payload of a [`SessionMessage`], reusing the existing streaming
/// message types verbatim.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SessionPayload {
    /// The frame-shaped transcript entry, decoded through the tagged
    /// `Message` deserializer.
    Message(Message),
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(
        clippy::panic,
        reason = "test failure signal via panic on a match miss"
    )]

    use super::*;

    #[test]
    fn session_message_holds_typed_assistant_payload() {
        let entry = serde_json::json!({
            "type": "assistant",
            "message": {"content": [{"type": "text", "text": "hi"}]}
        });
        let payload = SessionPayload::Message(serde_json::from_value(entry).expect("decode"));
        let SessionPayload::Message(Message::Assistant(a)) = payload else {
            panic!("expected an assistant payload");
        };
        assert_eq!(a.content.len(), 1);
    }
}

/// One parsed transcript entry: the fields the flat contract needs plus the
/// raw value for final payload decode.
#[derive(Clone, Debug)]
pub(crate) struct TranscriptEntry {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub kind: String,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub is_sidechain: bool,
    pub is_meta: bool,
    pub has_team_name: bool,
    pub raw: serde_json::Value,
}

impl TranscriptEntry {
    fn from_value(uuid: String, kind: String, v: serde_json::Value) -> Self {
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let b = |k: &str| {
            v.get(k)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        };
        Self {
            uuid,
            parent_uuid: s("parentUuid"),
            kind,
            timestamp: s("timestamp").unwrap_or_default(),
            session_id: s("sessionId"),
            is_sidechain: b("isSidechain"),
            is_meta: b("isMeta"),
            has_team_name: v.get("teamName").is_some_and(|t| !t.is_null()),
            raw: v,
        }
    }
}

/// Parse a raw transcript buffer into message entries (`user`/`assistant`/
/// `system` with a string `uuid`).
pub(crate) fn parse_transcript(bytes: &[u8]) -> Vec<TranscriptEntry> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    for line in text.split('\n') {
        let line = line.trim_start();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let kind = v.get("type").and_then(|t| t.as_str());
        let uuid = v.get("uuid").and_then(|u| u.as_str());
        if let (Some(k), Some(id)) = (kind, uuid)
            && matches!(k, "user" | "assistant" | "system")
        {
            out.push(TranscriptEntry::from_value(
                id.to_string(),
                k.to_string(),
                v,
            ));
        }
    }
    out
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn keeps_message_entries_with_string_uuid_only() {
        let jsonl = concat!(
            r#"{"type":"user","uuid":"u1","parentUuid":null,"message":{"content":"hi"}}"#,
            "\n",
            r#"{"type":"assistant","uuid":"a1","parentUuid":"u1","message":{"content":[]}}"#,
            "\n",
            r#"{"type":"progress","uuid":"p1"}"#,
            "\n", // not a message → dropped
            r#"{"type":"user"}"#,
            "\n",          // no uuid → dropped
            "   \n",       // blank → dropped
            r#"not json"#, // unparseable → dropped
        );
        let entries = parse_transcript(jsonl.as_bytes());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].uuid, "u1");
        assert_eq!(entries[1].parent_uuid.as_deref(), Some("u1"));
    }
}

/// Dedup entries by uuid: keep first-appearance order, latest record wins.
pub(crate) fn dedup_by_uuid(entries: Vec<TranscriptEntry>) -> Vec<TranscriptEntry> {
    let mut pos: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<TranscriptEntry> = Vec::new();
    for e in entries {
        if let Some(&i) = pos.get(&e.uuid) {
            out[i] = e;
        } else {
            pos.insert(e.uuid.clone(), out.len());
            out.push(e);
        }
    }
    out
}

#[cfg(test)]
mod dedup_tests {
    use super::*;

    fn entry(uuid: &str, kind: &str, marker: &str) -> TranscriptEntry {
        let v = serde_json::json!({
            "type": kind, "uuid": uuid, "parentUuid": null,
            "message": {"content": marker}
        });
        TranscriptEntry::from_value(uuid.to_string(), kind.to_string(), v)
    }

    #[test]
    fn keeps_last_value_at_first_position() {
        let entries = vec![
            entry("u1", "user", "old"),
            entry("u2", "user", "b"),
            entry("u1", "user", "new"), // same uuid, later record
        ];
        let out = dedup_by_uuid(entries);
        let ids: Vec<&str> = out.iter().map(|e| e.uuid.as_str()).collect();
        assert_eq!(ids, vec!["u1", "u2"], "first-appearance order preserved");
        assert_eq!(
            out[0].raw["message"]["content"], "new",
            "latest record wins"
        );
    }
}

/// Whether an entry passes the message filter: `user`/`assistant` always,
/// `system` only when `include_system`, and never a meta, sidechain, or
/// team-tagged entry.
fn passes_filter(e: &TranscriptEntry, include_system: bool) -> bool {
    let kind_ok = match e.kind.as_str() {
        "user" | "assistant" => true,
        "system" => include_system,
        _ => false,
    };
    kind_ok && !e.is_meta && !e.is_sidechain && !e.has_team_name
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    fn entry(kind: &str, meta: bool, sidechain: bool) -> TranscriptEntry {
        let v = serde_json::json!({
            "type": kind, "uuid": "x", "isMeta": meta, "isSidechain": sidechain,
            "message": {"content": []}
        });
        TranscriptEntry::from_value("x".into(), kind.into(), v)
    }

    #[test]
    fn keeps_user_assistant_gates_system_drops_meta_and_sidechain() {
        assert!(passes_filter(&entry("user", false, false), false));
        assert!(passes_filter(&entry("assistant", false, false), false));
        assert!(!passes_filter(&entry("system", false, false), false));
        assert!(passes_filter(&entry("system", false, false), true));
        assert!(!passes_filter(&entry("user", true, false), false)); // meta
        assert!(!passes_filter(&entry("user", false, true), false)); // sidechain
    }
}

/// Map filtered entries to `SessionMessage`s and paginate. Entries that fail
/// to decode through `Message` (never expected post-filter) are skipped.
pub(crate) fn assemble_messages(
    entries: Vec<TranscriptEntry>,
    include_system: bool,
    limit: Option<usize>,
    offset: usize,
) -> Vec<SessionMessage> {
    let mut mapped: Vec<SessionMessage> = Vec::new();
    for e in entries {
        if !passes_filter(&e, include_system) {
            continue;
        }
        let Ok(message) = serde_json::from_value::<Message>(e.raw.clone()) else {
            continue;
        };
        if matches!(
            message,
            Message::User(_) | Message::Assistant(_) | Message::System(_)
        ) {
            let session_id = e.session_id.unwrap_or_default();
            mapped.push(SessionMessage {
                uuid: e.uuid,
                parent_uuid: e.parent_uuid,
                session_id: SessionId::new(session_id),
                payload: SessionPayload::Message(message),
                timestamp: e.timestamp,
            });
        }
    }
    let start = offset.min(mapped.len());
    let mut paged = mapped.split_off(start);
    if let Some(l) = limit.filter(|l| *l > 0) {
        paged.truncate(l);
    }
    paged
}

#[cfg(test)]
mod assemble_tests {
    use super::*;
    use crate::agent::message::Message;

    fn entry(
        uuid: &str,
        parent: Option<&str>,
        kind: &str,
        extra: &serde_json::Value,
    ) -> TranscriptEntry {
        let mut v = serde_json::json!({
            "type": kind, "uuid": uuid, "parentUuid": parent, "message": {"content": []}
        });
        if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
            for (k, val) in e {
                o.insert(k.clone(), val.clone());
            }
        }
        TranscriptEntry::from_value(uuid.to_string(), kind.to_string(), v)
    }

    #[test]
    fn maps_envelope_including_parent_uuid_and_paginates() {
        let entries = vec![
            entry(
                "u1",
                None,
                "user",
                &serde_json::json!({"sessionId":"s1","timestamp":"t0"}),
            ),
            entry(
                "a1",
                Some("u1"),
                "assistant",
                &serde_json::json!({"sessionId":"s1"}),
            ),
            entry("u2", Some("a1"), "user", &serde_json::json!({})),
        ];
        let all = assemble_messages(entries.clone(), false, None, 0);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].uuid, "u1");
        assert_eq!(all[0].session_id.as_str(), "s1");
        assert_eq!(all[0].timestamp, "t0");
        assert_eq!(all[1].parent_uuid.as_deref(), Some("u1"));
        assert!(matches!(
            all[0].payload,
            SessionPayload::Message(Message::User(_))
        ));

        let paged = assemble_messages(entries, false, Some(1), 1);
        assert_eq!(paged.len(), 1);
        assert_eq!(paged[0].uuid, "a1", "offset 1, limit 1");
    }
}

/// Encode a `renameSession` record (`{type:"custom-title",customTitle,sessionId}` + `\n`).
pub(crate) fn encode_rename_record(session_id: &str, title: &str) -> String {
    #[derive(Serialize)]
    struct RenameRecord<'a> {
        #[serde(rename = "type")]
        ty: &'a str,
        #[serde(rename = "customTitle")]
        custom_title: &'a str,
        #[serde(rename = "sessionId")]
        session_id: &'a str,
    }
    #[expect(
        clippy::expect_used,
        reason = "a struct of &str fields is always representable as JSON"
    )]
    let mut s = serde_json::to_string(&RenameRecord {
        ty: "custom-title",
        custom_title: title,
        session_id,
    })
    .expect("record serializes");
    s.push('\n');
    s
}

/// Encode a `tagSession` record (`{type:"tag",tag,sessionId}` + `\n`).
pub(crate) fn encode_tag_record(session_id: &str, tag: &str) -> String {
    #[derive(Serialize)]
    struct TagRecord<'a> {
        #[serde(rename = "type")]
        ty: &'a str,
        tag: &'a str,
        #[serde(rename = "sessionId")]
        session_id: &'a str,
    }
    #[expect(
        clippy::expect_used,
        reason = "a struct of &str fields is always representable as JSON"
    )]
    let mut s = serde_json::to_string(&TagRecord {
        ty: "tag",
        tag,
        session_id,
    })
    .expect("record serializes");
    s.push('\n');
    s
}

#[cfg(test)]
mod record_tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::*;

    #[test]
    fn rename_record_has_type_title_and_id() {
        let r = encode_rename_record("f28ced56-9bd4-41f8-a37d-2a496c7d0e35", "My Title");
        assert!(r.ends_with('\n'));
        let v: serde_json::Value = serde_json::from_str(r.trim_end()).expect("valid json");
        assert_eq!(v["type"], "custom-title");
        assert_eq!(v["customTitle"], "My Title");
        assert_eq!(v["sessionId"], "f28ced56-9bd4-41f8-a37d-2a496c7d0e35");
    }

    #[test]
    fn tag_record_has_type_tag_and_id() {
        let r = encode_tag_record("f28ced56-9bd4-41f8-a37d-2a496c7d0e35", "");
        let v: serde_json::Value = serde_json::from_str(r.trim_end()).expect("valid json");
        assert_eq!(v["type"], "tag");
        assert_eq!(v["tag"], "");
    }
}
