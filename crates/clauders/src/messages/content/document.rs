//! Document input block (PDF and text) for the request content-block union.
//!
//! Exists as its own file so the document block, its source variants, and
//! its citation-enable config are scoped apart from the other request
//! blocks. Referenced only by [`super::ContentBlockParam::Document`].

/// Media type of a base64-encoded PDF document (single-valued).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PdfMediaType {
    /// `application/pdf`.
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
}

/// Media type of a plain-text document source (single-valued).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlainTextMediaType {
    /// `text/plain`.
    #[serde(rename = "text/plain")]
    TextPlain,
}

/// Toggle for whether the model may cite this document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CitationsConfig {
    /// When `true`, responses may carry citations into this document.
    pub enabled: bool,
}

/// Source of a document.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    /// Inline base64-encoded PDF bytes.
    Base64 {
        /// The document's media type (`application/pdf`).
        media_type: PdfMediaType,
        /// The base64-encoded PDF bytes.
        data: String,
    },
    /// Inline plain text treated as a document.
    Text {
        /// The document's media type (`text/plain`).
        media_type: PlainTextMediaType,
        /// The plain-text document body.
        data: String,
    },
    /// A document assembled from embedded content (text or blocks).
    ///
    /// The embedded content is retained as raw JSON — this uncommon
    /// embedding path is not typed further in this release.
    Content {
        /// The embedded content: a JSON string or array of source blocks.
        content: serde_json::Value,
    },
    /// A URL the API fetches the PDF from.
    Url {
        /// The document URL.
        url: String,
    },
}

/// A document content block a caller sends for PDF/text input.
///
/// # Examples
///
/// ```
/// use clauders::messages::content::document::{DocumentBlock, DocumentSource, PdfMediaType};
/// let block = DocumentBlock {
///     source: DocumentSource::Base64 { media_type: PdfMediaType::ApplicationPdf, data: "JVBER".into() },
///     cache_control: None,
///     citations: None,
///     context: None,
///     title: Some("report.pdf".into()),
/// };
/// let j = serde_json::to_value(&block).unwrap();
/// assert_eq!(j["source"]["media_type"], "application/pdf");
/// assert_eq!(j["title"], "report.pdf");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DocumentBlock {
    /// Where the document comes from.
    pub source: DocumentSource,
    /// Optional cache breakpoint for this block.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cache_control: Option<crate::types::CacheControl>,
    /// Enable citations into this document.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub citations: Option<CitationsConfig>,
    /// Optional operator context string for the document.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context: Option<String>,
    /// Optional document title.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn base64_pdf_serializes_with_media_type() {
        let block = DocumentBlock {
            source: DocumentSource::Base64 {
                media_type: PdfMediaType::ApplicationPdf,
                data: "JVBER".into(),
            },
            cache_control: None,
            citations: None,
            context: None,
            title: None,
        };
        let j = serde_json::to_value(&block).unwrap();
        assert_eq!(j["source"]["type"], "base64");
        assert_eq!(j["source"]["media_type"], "application/pdf");
        assert!(j.get("title").is_none(), "absent optionals are omitted");
    }

    #[test]
    fn url_pdf_round_trips_with_citations() {
        let original = DocumentBlock {
            source: DocumentSource::Url {
                url: "https://example.com/x.pdf".into(),
            },
            cache_control: None,
            citations: Some(CitationsConfig { enabled: true }),
            context: None,
            title: None,
        };
        let j = serde_json::to_string(&original).unwrap();
        let back: DocumentBlock = serde_json::from_str(&j).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn plain_text_source_round_trips() {
        let original = DocumentSource::Text {
            media_type: PlainTextMediaType::TextPlain,
            data: "hello".into(),
        };
        let j = serde_json::to_string(&original).unwrap();
        let back: DocumentSource = serde_json::from_str(&j).unwrap();
        assert_eq!(back, original);
    }
}
