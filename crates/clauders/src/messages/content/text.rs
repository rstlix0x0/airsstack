//! Leaf content-block structs shared by both the request and response unions.
//!
//! Exists as its own file so the plain-data block shapes (`TextBlock`,
//! `ThinkingBlock`) are defined once and reused by [`super::ContentBlock`]
//! (response) and [`super::ContentBlockParam`] (request), rather than
//! duplicated per direction.

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
    /// Optional cache breakpoint for this block.
    ///
    /// When set, this block marks a prompt-caching boundary.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cache_control: Option<crate::types::CacheControl>,
    /// Citations backing this text, if the response carried any.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub citations: Option<Vec<crate::messages::content::citation::TextCitation>>,
}

impl TextBlock {
    /// Construct a `TextBlock` from any string-like value.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self {
            text: s.into(),
            cache_control: None,
            citations: None,
        }
    }

    /// Attach a cache breakpoint to this block.
    ///
    /// Marks this text block as a prompt-caching boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::messages::TextBlock;
    /// use clauders::types::CacheControl;
    /// let b = TextBlock::new("You are terse.").with_cache(CacheControl::ephemeral());
    /// let j = serde_json::to_string(&b).unwrap();
    /// assert!(j.contains("\"cache_control\":{\"type\":\"ephemeral\"}"));
    /// ```
    #[must_use]
    pub const fn with_cache(mut self, cc: crate::types::CacheControl) -> Self {
        self.cache_control = Some(cc);
        self
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
    fn text_block_with_cache_serializes_field() {
        use crate::types::CacheControl;
        let b = TextBlock::new("hi").with_cache(CacheControl::ephemeral());
        let j = serde_json::to_string(&b).unwrap();
        assert_eq!(j, r#"{"text":"hi","cache_control":{"type":"ephemeral"}}"#);
    }

    #[test]
    fn text_block_without_cache_omits_field() {
        let b = TextBlock::new("hi");
        let j = serde_json::to_string(&b).unwrap();
        assert_eq!(j, r#"{"text":"hi"}"#);
    }

    #[test]
    fn text_block_with_cache_round_trips() {
        use crate::types::CacheControl;
        let original = TextBlock::new("hello").with_cache(CacheControl::ephemeral());
        let j = serde_json::to_string(&original).unwrap();
        let back: TextBlock = serde_json::from_str(&j).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn text_block_decodes_citations() {
        use crate::messages::content::citation::TextCitation;
        let json = r#"{"text":"hi","citations":[
            {"type":"page_location","cited_text":"x","document_index":0,
             "start_page_number":1,"end_page_number":2}]}"#;
        let b: TextBlock = serde_json::from_str(json).unwrap();
        assert_eq!(b.citations.as_ref().unwrap().len(), 1);
        assert!(matches!(
            b.citations.unwrap()[0],
            TextCitation::PageLocation { .. }
        ));
    }

    #[test]
    fn text_block_without_citations_omits_field() {
        let b = TextBlock::new("hi");
        let j = serde_json::to_string(&b).unwrap();
        assert_eq!(j, r#"{"text":"hi"}"#);
    }
}
