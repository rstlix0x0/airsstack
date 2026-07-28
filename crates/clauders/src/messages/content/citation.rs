//! Citation-location shapes carried by a [`super::TextBlock`].
//!
//! Exists as its own file so the five citation-location kinds are scoped
//! apart from the text block itself. Server-decoded: a kind this release
//! does not model is retained in [`TextCitation::Unknown`].

/// A single citation into a source document or search result.
///
/// Discriminated on the wire `type`. A kind this release does not model
/// lands in [`TextCitation::Unknown`] with its payload retained, rather
/// than failing the enclosing text block.
///
/// # Examples
///
/// ```
/// use clauders::messages::TextCitation;
/// let json = r#"{"type":"page_location","cited_text":"x","document_index":0,
///     "start_page_number":1,"end_page_number":2}"#;
/// let c: TextCitation = serde_json::from_str(json).unwrap();
/// assert!(matches!(c, TextCitation::PageLocation { .. }));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TextCitation {
    /// A character-range citation into a document.
    CharLocation {
        /// The exact text cited.
        cited_text: String,
        /// 0-based index of the cited document among request documents.
        document_index: u32,
        /// The cited document's title, if any.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        document_title: Option<String>,
        /// 0-based start character offset (inclusive).
        start_char_index: u32,
        /// End character offset (exclusive).
        end_char_index: u32,
        /// The cited document's file id, if any.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        file_id: Option<String>,
    },
    /// A page-range citation into a document.
    PageLocation {
        /// The exact text cited.
        cited_text: String,
        /// 0-based index of the cited document among request documents.
        document_index: u32,
        /// The cited document's title, if any.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        document_title: Option<String>,
        /// 1-based start page number (inclusive).
        start_page_number: u32,
        /// End page number (exclusive).
        end_page_number: u32,
        /// The cited document's file id, if any.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        file_id: Option<String>,
    },
    /// A content-block-range citation into a document.
    ContentBlockLocation {
        /// The exact text cited.
        cited_text: String,
        /// 0-based index of the cited document among request documents.
        document_index: u32,
        /// The cited document's title, if any.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        document_title: Option<String>,
        /// 0-based start block index (inclusive).
        start_block_index: u32,
        /// End block index (exclusive).
        end_block_index: u32,
        /// The cited document's file id, if any.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        file_id: Option<String>,
    },
    /// A citation into a server-side web-search result.
    WebSearchResultLocation {
        /// The exact text cited.
        cited_text: String,
        /// The result URL.
        url: String,
        /// The result title, if any.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        title: Option<String>,
        /// Opaque encrypted index of the cited result.
        encrypted_index: String,
    },
    /// A citation into a caller-supplied `search_result` block.
    SearchResultLocation {
        /// The exact text cited.
        cited_text: String,
        /// The result's source identifier.
        source: String,
        /// The result title, if any.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        title: Option<String>,
        /// 0-based index of the cited search result among request results.
        search_result_index: u32,
        /// 0-based start block index (inclusive).
        start_block_index: u32,
        /// End block index (exclusive).
        end_block_index: u32,
    },
    /// A citation kind this release does not model; raw payload retained.
    #[serde(untagged)]
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
    fn char_location_decodes_all_fields() {
        let json = r#"{"type":"char_location","cited_text":"hi","document_index":2,
            "document_title":"Doc","start_char_index":5,"end_char_index":7,"file_id":"file_1"}"#;
        let c: TextCitation = serde_json::from_str(json).unwrap();
        assert_eq!(
            c,
            TextCitation::CharLocation {
                cited_text: "hi".into(),
                document_index: 2,
                document_title: Some("Doc".into()),
                start_char_index: 5,
                end_char_index: 7,
                file_id: Some("file_1".into()),
            }
        );
    }

    #[test]
    fn web_search_result_location_decodes() {
        let json = r#"{"type":"web_search_result_location","cited_text":"x",
            "url":"https://e.com","encrypted_index":"enc"}"#;
        let c: TextCitation = serde_json::from_str(json).unwrap();
        assert!(matches!(c, TextCitation::WebSearchResultLocation { .. }));
    }

    #[test]
    fn search_result_location_round_trips() {
        let original = TextCitation::SearchResultLocation {
            cited_text: "x".into(),
            source: "src".into(),
            title: None,
            search_result_index: 0,
            start_block_index: 0,
            end_block_index: 1,
        };
        let j = serde_json::to_string(&original).unwrap();
        let back: TextCitation = serde_json::from_str(&j).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn unknown_citation_kind_retains_payload() {
        let json = r#"{"type":"future_location","cited_text":"x"}"#;
        let c: TextCitation = serde_json::from_str(json).unwrap();
        match c {
            TextCitation::Unknown(v) => assert_eq!(v["type"], "future_location"),
            other => panic!("wrong variant: {other:?}"),
        }
    }
}
