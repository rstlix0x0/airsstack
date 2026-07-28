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
    /// An image block for vision input.
    Image(crate::messages::content::image::ImageBlock),
    /// A document block for PDF/text input.
    Document(crate::messages::content::document::DocumentBlock),
}

/// Error returned when a response-only [`ContentBlock`] cannot be sent as a
/// [`ContentBlockParam`].
///
/// A response block kind with no request-union counterpart — redacted
/// thinking, a server-tool invocation or result, a container upload, or a
/// kind this release does not model — cannot be echoed back into a request.
///
/// [`ContentBlock`]: crate::messages::ContentBlock
///
/// # Examples
///
/// ```
/// use clauders::messages::{ContentBlock, ContentBlockParam};
/// let unknown = ContentBlock::Unknown(serde_json::json!({"type":"server_tool_use"}));
/// let err = ContentBlockParam::try_from(unknown).unwrap_err();
/// assert_eq!(err.block_type(), "server_tool_use");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("content block of type `{block_type}` cannot be sent in a request")]
pub struct UnsendableBlock {
    block_type: String,
}

impl UnsendableBlock {
    /// The wire `type` tag of the block that could not be sent.
    #[must_use]
    pub fn block_type(&self) -> &str {
        &self.block_type
    }

    /// Construct from a static wire `type` for a response-only block variant.
    fn of(block_type: &str) -> Self {
        Self {
            block_type: block_type.to_owned(),
        }
    }
}

impl TryFrom<crate::messages::content::ContentBlock> for ContentBlockParam {
    type Error = UnsendableBlock;

    fn try_from(block: crate::messages::content::ContentBlock) -> Result<Self, Self::Error> {
        use crate::messages::content::ContentBlock;
        match block {
            ContentBlock::Text(t) => Ok(Self::Text(t)),
            ContentBlock::Thinking(t) => Ok(Self::Thinking(t)),
            ContentBlock::ToolUse(t) => Ok(Self::ToolUse(t)),
            ContentBlock::RedactedThinking(_) => Err(UnsendableBlock::of("redacted_thinking")),
            ContentBlock::ServerToolUse(_) => Err(UnsendableBlock::of("server_tool_use")),
            ContentBlock::WebSearchToolResult(_) => {
                Err(UnsendableBlock::of("web_search_tool_result"))
            }
            ContentBlock::WebFetchToolResult(_) => {
                Err(UnsendableBlock::of("web_fetch_tool_result"))
            }
            ContentBlock::CodeExecutionToolResult(_) => {
                Err(UnsendableBlock::of("code_execution_tool_result"))
            }
            ContentBlock::BashCodeExecutionToolResult(_) => {
                Err(UnsendableBlock::of("bash_code_execution_tool_result"))
            }
            ContentBlock::TextEditorCodeExecutionToolResult(_) => Err(UnsendableBlock::of(
                "text_editor_code_execution_tool_result",
            )),
            ContentBlock::ToolSearchToolResult(_) => {
                Err(UnsendableBlock::of("tool_search_tool_result"))
            }
            ContentBlock::ContainerUpload(_) => Err(UnsendableBlock::of("container_upload")),
            ContentBlock::Unknown(v) => Err(UnsendableBlock {
                block_type: v
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned(),
            }),
        }
    }
}

impl ContentBlockParam {
    /// Convert a response message's content blocks into request blocks.
    ///
    /// Shared block kinds (`text`, `thinking`, `tool_use`) convert; a
    /// response-only block fails the whole batch with the first
    /// [`UnsendableBlock`] encountered. This is all-or-nothing by design —
    /// silently dropping a block the caller meant to send would be the kind
    /// of quiet data loss the typed API exists to prevent.
    ///
    /// # Errors
    ///
    /// Returns [`UnsendableBlock`] naming the wire `type` of the first block
    /// that has no request-union counterpart.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::messages::{ContentBlock, ContentBlockParam, TextBlock};
    /// let blocks = vec![ContentBlock::Text(TextBlock::new("hi"))];
    /// let params = ContentBlockParam::try_from_response(blocks).unwrap();
    /// assert_eq!(params.len(), 1);
    /// ```
    pub fn try_from_response(
        blocks: Vec<crate::messages::content::ContentBlock>,
    ) -> Result<Vec<Self>, UnsendableBlock> {
        blocks.into_iter().map(Self::try_from).collect()
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

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

    #[test]
    fn shared_blocks_convert_from_response_union() {
        use crate::messages::content::ContentBlock;
        let text = ContentBlock::Text(TextBlock::new("hi"));
        let param = ContentBlockParam::try_from(text).expect("text is sendable");
        assert_eq!(param, ContentBlockParam::Text(TextBlock::new("hi")));
    }

    #[test]
    fn unknown_response_block_is_unsendable() {
        use crate::messages::content::ContentBlock;
        let unknown = ContentBlock::Unknown(serde_json::json!({"type":"redacted_thinking"}));
        let err = ContentBlockParam::try_from(unknown).expect_err("unknown is not sendable");
        assert_eq!(err.block_type(), "redacted_thinking");
    }

    #[test]
    fn try_from_response_vec_fails_on_first_unsendable() {
        use crate::messages::content::ContentBlock;
        let blocks = vec![
            ContentBlock::Text(TextBlock::new("ok")),
            ContentBlock::Unknown(serde_json::json!({"type":"server_tool_use"})),
        ];
        let err = ContentBlockParam::try_from_response(blocks)
            .expect_err("a mixed vec with an unsendable block fails");
        assert_eq!(err.block_type(), "server_tool_use");
    }

    #[test]
    fn image_param_serializes_with_type_tag() {
        use crate::messages::content::image::{ImageBlock, ImageMediaType, ImageSource};
        let block = ContentBlockParam::Image(ImageBlock {
            source: ImageSource::Base64 {
                media_type: ImageMediaType::Png,
                data: "AAAA".into(),
            },
            cache_control: None,
        });
        let j = serde_json::to_value(&block).unwrap();
        assert_eq!(j["type"], "image");
        assert_eq!(j["source"]["type"], "base64");
    }

    #[test]
    fn document_param_serializes_with_type_tag() {
        use crate::messages::content::document::{DocumentBlock, DocumentSource, PdfMediaType};
        let block = ContentBlockParam::Document(DocumentBlock {
            source: DocumentSource::Base64 {
                media_type: PdfMediaType::ApplicationPdf,
                data: "JVBER".into(),
            },
            cache_control: None,
            citations: None,
            context: None,
            title: None,
        });
        let j = serde_json::to_value(&block).unwrap();
        assert_eq!(j["type"], "document");
        assert_eq!(j["source"]["type"], "base64");
    }

    #[test]
    fn server_tool_use_block_is_unsendable() {
        use crate::messages::content::ContentBlock;
        use crate::messages::content::server_tool::{ServerToolName, ServerToolUseBlock};
        let block = ContentBlock::ServerToolUse(ServerToolUseBlock {
            id: "srvtoolu_1".into(),
            name: ServerToolName::WebSearch,
            input: serde_json::json!({}),
            caller: None,
        });
        let err = ContentBlockParam::try_from(block).unwrap_err();
        assert_eq!(err.block_type(), "server_tool_use");
    }

    #[test]
    fn try_from_response_vec_succeeds_when_all_shared() {
        use crate::messages::content::ContentBlock;
        let blocks = vec![
            ContentBlock::Text(TextBlock::new("a")),
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "b".into(),
                signature: None,
            }),
        ];
        let params = ContentBlockParam::try_from_response(blocks).expect("all shared");
        assert_eq!(params.len(), 2);
    }
}
