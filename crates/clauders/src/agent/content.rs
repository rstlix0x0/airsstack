//! Content blocks that make up an assistant or user message.

use serde::{Deserialize, Serialize};

/// One content block within a message.
///
/// Exhaustive: the compiler forces consumers to handle every block kind, so
/// a new message shape cannot be silently dropped. Unknown *fields* within a
/// known block are tolerated (forward-compat); an unknown *block type*
/// surfaces as a deserialize error the reader maps to `AgentError::Protocol`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "serde_json::Value does not implement Eq; cannot derive it for this enum"
)]
#[serde(tag = "type", rename_all = "snake_case")]
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
}
