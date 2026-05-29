//! Content block types for Messages API request and response bodies.
//!
//! Exists as its own module so each content-block shape can be extended
//! independently without touching response decoding or request assembly.
//!
//! Responsibilities:
//! - Define [`ContentBlock`], the tagged union dispatching on `"type"`.
//! - Define [`TextBlock`] (plain text) and [`ThinkingBlock`] (extended
//!   thinking output with an optional signature).
//!
//! Not responsible for:
//! - Request construction or response decoding — those live in `request.rs`
//!   and `response.rs` respectively.
//! - Tool-use blocks — a separate type in a future extension.

/// Tagged union of content block shapes returned or accepted by the Messages API.
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
pub enum ContentBlock {
    /// A plain-text content block.
    Text(TextBlock),
    /// Extended thinking output, optionally carrying a verification signature.
    Thinking(ThinkingBlock),
}

/// Plain-text content block.
///
/// # Examples
///
/// ```
/// use clauders::messages::TextBlock;
/// let b = TextBlock::new("hello");
/// assert_eq!(b.text, "hello");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TextBlock {
    /// The text content of this block.
    pub text: String,
}

impl TextBlock {
    /// Construct a `TextBlock` from any string-like value.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self { text: s.into() }
    }
}

/// Extended thinking output block, optionally carrying a verification signature.
///
/// The `signature` field is omitted from serialized output when absent.
///
/// # Examples
///
/// ```
/// use clauders::messages::ThinkingBlock;
/// let b = ThinkingBlock { thinking: "42".into(), signature: None };
/// let j = serde_json::to_string(&b).unwrap();
/// assert_eq!(j, r#"{"thinking":"42"}"#);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ThinkingBlock {
    /// The thinking text produced by the model.
    pub thinking: String,
    /// Optional cryptographic signature for verifying thinking authenticity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
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
}
