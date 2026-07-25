//! The response content-block union.
//!
//! Exists as its own file so the block kinds the API *returns* are defined
//! apart from the request union in [`super::param`]. Leaf structs come from
//! [`super::text`] and [`crate::messages::tools`].

use crate::messages::content::text::{TextBlock, ThinkingBlock};

/// Tagged union of content block shapes the Messages API returns in a response.
///
/// The `"type"` field in the JSON wire format acts as the discriminant;
/// serde's `tag = "type"` maps it to the enum variant.
///
/// # Examples
///
/// ```
/// use clauders::messages::{ContentBlock, TextBlock};
/// let block = ContentBlock::Text(TextBlock::new("hello"));
/// let j = serde_json::to_string(&block).unwrap();
/// assert_eq!(j, r#"{"type":"text","text":"hello"}"#);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContentBlock {
    /// A plain-text content block.
    Text(TextBlock),
    /// Extended thinking output, optionally carrying a verification signature.
    Thinking(ThinkingBlock),
    /// A tool invocation produced by the model.
    ToolUse(crate::messages::tools::ToolUseBlock),
    /// A tool result supplied by the caller in response to a tool invocation.
    ToolResult(crate::messages::tools::ToolResultBlock),
    /// A block kind this SDK release does not model.
    ///
    /// The API returns block kinds this release has no typed shape for —
    /// server-tool invocations and their results, redacted thinking, and
    /// container uploads among them. The raw JSON object is retained here so
    /// the surrounding response stays decodable and the block can still be
    /// inspected.
    ///
    /// This variant is **deserialize-only**. Attempting to serialize it — by
    /// putting it back into a request — is an error rather than a silent
    /// round-trip of a block this SDK does not understand.
    #[serde(untagged, skip_serializing)]
    Unknown(serde_json::Value),
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::panic,
        reason = "test-only panic on a wrong-variant match; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn text_block_serializes_with_tag() {
        let block = ContentBlock::Text(TextBlock::new("hi"));
        let j = serde_json::to_string(&block).unwrap();
        assert_eq!(j, r#"{"type":"text","text":"hi"}"#);
    }

    #[test]
    fn thinking_block_omits_optional_signature() {
        let block = ContentBlock::Thinking(ThinkingBlock {
            thinking: "deep thought".into(),
            signature: None,
        });
        let j = serde_json::to_string(&block).unwrap();
        assert_eq!(j, r#"{"type":"thinking","thinking":"deep thought"}"#);
    }

    #[test]
    fn content_block_round_trips_via_serde() {
        let original = ContentBlock::Text(TextBlock::new("hello"));
        let j = serde_json::to_string(&original).unwrap();
        let back: ContentBlock = serde_json::from_str(&j).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn unknown_block_type_decodes_with_payload_retained() {
        let json = r#"{"type":"server_tool_use","id":"srvtoolu_01","name":"web_search"}"#;
        let block: ContentBlock = serde_json::from_str(json).unwrap();
        match block {
            ContentBlock::Unknown(v) => {
                assert_eq!(v["type"], "server_tool_use");
                assert_eq!(v["id"], "srvtoolu_01");
                assert_eq!(v["name"], "web_search");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn known_block_types_still_decode_with_unknown_arm_present() {
        let block: ContentBlock = serde_json::from_str(r#"{"type":"text","text":"hi"}"#).unwrap();
        assert_eq!(block, ContentBlock::Text(TextBlock::new("hi")));
    }
}
