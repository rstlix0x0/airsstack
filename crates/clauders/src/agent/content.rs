//! Content blocks that make up an assistant or user message.

use serde::{Deserialize, Serialize};

/// One content block within a message.
///
/// A block kind this release does not model is captured verbatim by
/// [`ContentBlock::Unknown`] rather than failing the enclosing message, so a
/// frame mixing modelled and unmodelled blocks keeps the modelled ones.
///
/// Derives `PartialEq` but not `Eq`: `serde_json::Value` does not implement
/// `Eq`, and several variants carry one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContentBlock {
    /// Plain assistant/user text.
    Text {
        /// The text payload.
        text: String,
    },
    /// Extended-thinking text.
    Thinking {
        /// The thinking text.
        thinking: String,
    },
    /// A request by the model to invoke a tool.
    ToolUse {
        /// Unique id correlating this call with its result.
        id: String,
        /// Tool name.
        name: String,
        /// Tool input arguments (opaque JSON).
        input: serde_json::Value,
    },
    /// The result of a tool invocation.
    ToolResult {
        /// Id of the `tool_use` this result answers.
        tool_use_id: String,
        /// Result content (opaque JSON: string or block array).
        #[serde(default)]
        content: serde_json::Value,
        /// Whether the tool reported an error.
        #[serde(default)]
        is_error: bool,
    },
    /// A server-side tool invocation surfaced by the binary.
    ServerToolUse {
        /// Unique id of the server tool call.
        id: String,
        /// Server tool name.
        name: String,
        /// Server tool input (opaque JSON).
        input: serde_json::Value,
    },
    /// A block kind this SDK release does not model.
    ///
    /// The binary emits kinds with no typed shape here — `redacted_thinking`,
    /// `web_search_tool_result`, `mcp_tool_use`, `mcp_tool_result` and
    /// `container_upload` among them. The raw JSON object is retained so the
    /// surrounding message stays decodable.
    ///
    /// This also absorbs a *malformed* block of a kind that IS modelled: a
    /// `text` block whose `text` is not a string lands here rather than
    /// raising. That trade-off is deliberate — the alternative loses the
    /// whole message — but it means this variant is not proof that the
    /// binary sent something new.
    ///
    /// Deserialize-only. Serializing it is an error rather than a silent
    /// round-trip of a shape this release does not understand.
    #[serde(untagged, skip_serializing)]
    Unknown(serde_json::Value),
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::ContentBlock;

    #[test]
    fn deserializes_text_block() {
        let json = r#"{"type":"text","text":"hi"}"#;
        let block: ContentBlock = serde_json::from_str(json).expect("deserialize");
        assert!(matches!(block, ContentBlock::Text { text } if text == "hi"));
    }

    #[test]
    fn deserializes_tool_use_block() {
        let json = r#"{"type":"tool_use","id":"tu_1","name":"bash","input":{"cmd":"ls"}}"#;
        let block: ContentBlock = serde_json::from_str(json).expect("deserialize");
        match block {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "tu_1");
                assert_eq!(name, "bash");
                assert_eq!(input["cmd"], "ls");
            }
            other => panic!("expected ToolUse, got {other:?}"),
        }
    }

    #[test]
    fn tolerates_unknown_fields() {
        let json = r#"{"type":"text","text":"hi","brand_new_field":42}"#;
        let block: ContentBlock = serde_json::from_str(json).expect("tolerant deserialize");
        assert!(matches!(block, ContentBlock::Text { .. }));
    }

    #[test]
    fn unknown_block_type_is_captured_verbatim() {
        let json = r#"{"type":"redacted_thinking","data":"abc"}"#;
        let block: ContentBlock = serde_json::from_str(json).expect("deserialize");
        let ContentBlock::Unknown(value) = block else {
            panic!("expected Unknown, got {block:?}");
        };
        assert_eq!(value["type"], "redacted_thinking");
        assert_eq!(value["data"], "abc");
    }

    #[test]
    fn unknown_block_does_not_cost_its_siblings() {
        // Verbatim shape of an assistant frame's content array carrying one
        // modelled block and one this release does not model.
        let json = r#"[{"type":"text","text":"hi"},{"type":"redacted_thinking","data":"abc"}]"#;
        let blocks: Vec<ContentBlock> = serde_json::from_str(json).expect("deserialize");
        assert_eq!(blocks.len(), 2, "both blocks must survive");
        assert!(matches!(&blocks[0], ContentBlock::Text { text } if text == "hi"));
        assert!(matches!(blocks[1], ContentBlock::Unknown(_)));
    }

    #[test]
    fn unknown_block_cannot_be_serialized_back() {
        let block = ContentBlock::Unknown(serde_json::json!({"type":"redacted_thinking"}));
        let error = serde_json::to_string(&block).expect_err("must not serialize");
        assert!(
            error.to_string().contains("cannot be serialized"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn malformed_known_block_lands_in_unknown() {
        // Accepted trade-off, pinned deliberately: a wrong-typed field in a
        // block kind we DO model is captured rather than raised. Without this
        // test the behaviour would be discovered in production instead.
        let json = r#"{"type":"text","text":123}"#;
        let block: ContentBlock = serde_json::from_str(json).expect("deserialize");
        assert!(matches!(block, ContentBlock::Unknown(_)));
    }
}
