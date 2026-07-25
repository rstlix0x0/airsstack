//! The request content-block union.
//!
//! Exists as its own file so the block kinds a caller may *send* are defined
//! apart from the response union in [`super::block`]. This union is
//! request-authored: it is closed, with no `Unknown` fallback, because a
//! caller only ever produces block kinds the crate names.

use crate::messages::content::text::{TextBlock, ThinkingBlock};
use crate::messages::tools::{ToolResultBlock, ToolUseBlock};

/// Tagged union of content block shapes a caller may send in a request.
///
/// Closed by design: unlike [`super::ContentBlock`] there is no `Unknown`
/// arm, because every block sent is one the crate constructed.
///
/// # Examples
///
/// ```
/// use clauders::messages::content::{ContentBlockParam, TextBlock};
/// let block = ContentBlockParam::Text(TextBlock::new("hello"));
/// let j = serde_json::to_string(&block).unwrap();
/// assert_eq!(j, r#"{"type":"text","text":"hello"}"#);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContentBlockParam {
    /// A plain-text block.
    Text(TextBlock),
    /// An extended-thinking block echoed back into an assistant turn.
    Thinking(ThinkingBlock),
    /// A tool invocation echoed back into an assistant turn.
    ToolUse(ToolUseBlock),
    /// A tool result the caller supplies in a follow-up user turn.
    ToolResult(ToolResultBlock),
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn text_param_serializes_with_tag() {
        let block = ContentBlockParam::Text(TextBlock::new("hi"));
        let j = serde_json::to_string(&block).unwrap();
        assert_eq!(j, r#"{"type":"text","text":"hi"}"#);
    }

    #[test]
    fn tool_result_param_round_trips_via_serde() {
        use crate::messages::tools::ToolResultBlock;
        use crate::types::ToolUseId;
        let original = ContentBlockParam::ToolResult(ToolResultBlock::text(
            ToolUseId::new("toolu_01").unwrap(),
            "ok",
        ));
        let j = serde_json::to_string(&original).unwrap();
        let back: ContentBlockParam = serde_json::from_str(&j).unwrap();
        assert_eq!(back, original);
    }
}
