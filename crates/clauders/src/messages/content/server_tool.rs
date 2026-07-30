//! Response-only content blocks: server-tool invocations, their results,
//! redacted thinking, and container uploads.
//!
//! Exists as its own file so the block kinds the API *returns* but a caller
//! never sends are scoped apart from the request blocks. Each block models
//! its stable outer fields; the tool-specific result body is retained as
//! `serde_json::Value`. Referenced by [`super::ContentBlock`].

/// How a tool invocation was originated.
///
/// Server-decoded: a value this release does not model is retained in
/// [`ToolCaller::Unknown`] rather than failing the enclosing block.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolCaller {
    /// Tool invocation directly from the model.
    Direct,
    /// A server-tool invocation (code-execution generation 2025-08-25).
    #[serde(rename = "code_execution_20250825")]
    ServerTool {
        /// Identifier of the server tool that made the call.
        tool_id: String,
    },
    /// A server-tool invocation (code-execution generation 2026-01-20).
    #[serde(rename = "code_execution_20260120")]
    ServerTool20260120 {
        /// Identifier of the server tool that made the call.
        tool_id: String,
    },
    /// A caller kind this release does not model; raw payload retained.
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

/// The name of a server-side tool, a closed seven-value set.
///
/// A `server_tool_use` block whose `name` is outside this set fails to
/// decode as [`ServerToolUseBlock`] and is retained by the enclosing
/// [`super::ContentBlock::Unknown`] floor instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerToolName {
    /// `web_search`.
    WebSearch,
    /// `web_fetch`.
    WebFetch,
    /// `code_execution`.
    CodeExecution,
    /// `bash_code_execution`.
    BashCodeExecution,
    /// `text_editor_code_execution`.
    TextEditorCodeExecution,
    /// `tool_search_tool_regex`.
    ToolSearchToolRegex,
    /// `tool_search_tool_bm25`.
    ToolSearchToolBm25,
}

/// Redacted extended-thinking block: opaque encrypted thinking data.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedactedThinkingBlock {
    /// The opaque encrypted thinking payload.
    pub data: String,
}

/// A server-tool invocation the model produced.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ServerToolUseBlock {
    /// Server-assigned identifier for this invocation.
    pub id: String,
    /// Which server tool was called.
    pub name: ServerToolName,
    /// The tool input, retained as raw JSON.
    pub input: serde_json::Value,
    /// How the invocation was originated.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub caller: Option<ToolCaller>,
}

/// A file uploaded to the model's container.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContainerUploadBlock {
    /// Identifier of the uploaded file.
    pub file_id: String,
}

/// Result of a `web_search` server-tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WebSearchToolResultBlock {
    /// The [`ServerToolUseBlock::id`] this result answers.
    pub tool_use_id: String,
    /// The result payload, retained as raw JSON.
    pub content: serde_json::Value,
    /// How the originating invocation was made.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub caller: Option<ToolCaller>,
}

/// Result of a `web_fetch` server-tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WebFetchToolResultBlock {
    /// The [`ServerToolUseBlock::id`] this result answers.
    pub tool_use_id: String,
    /// The result payload, retained as raw JSON.
    pub content: serde_json::Value,
    /// How the originating invocation was made.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub caller: Option<ToolCaller>,
}

/// Result of a `code_execution` server-tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CodeExecutionToolResultBlock {
    /// The [`ServerToolUseBlock::id`] this result answers.
    pub tool_use_id: String,
    /// The result payload, retained as raw JSON.
    pub content: serde_json::Value,
}

/// Result of a `bash_code_execution` server-tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BashCodeExecutionToolResultBlock {
    /// The [`ServerToolUseBlock::id`] this result answers.
    pub tool_use_id: String,
    /// The result payload, retained as raw JSON.
    pub content: serde_json::Value,
}

/// Result of a `text_editor_code_execution` server-tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TextEditorCodeExecutionToolResultBlock {
    /// The [`ServerToolUseBlock::id`] this result answers.
    pub tool_use_id: String,
    /// The result payload, retained as raw JSON.
    pub content: serde_json::Value,
}

/// Result of a `tool_search` server-tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolSearchToolResultBlock {
    /// The [`ServerToolUseBlock::id`] this result answers.
    pub tool_use_id: String,
    /// The result payload, retained as raw JSON.
    pub content: serde_json::Value,
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
    fn direct_caller_round_trips() {
        let j = serde_json::to_string(&ToolCaller::Direct).unwrap();
        assert_eq!(j, r#"{"type":"direct"}"#);
        let back: ToolCaller = serde_json::from_str(&j).unwrap();
        assert_eq!(back, ToolCaller::Direct);
    }

    #[test]
    fn server_tool_callers_carry_tool_id() {
        let j = r#"{"type":"code_execution_20250825","tool_id":"srvtoolu_1"}"#;
        let c: ToolCaller = serde_json::from_str(j).unwrap();
        assert_eq!(
            c,
            ToolCaller::ServerTool {
                tool_id: "srvtoolu_1".into()
            }
        );

        let j2 = r#"{"type":"code_execution_20260120","tool_id":"srvtoolu_2"}"#;
        let c2: ToolCaller = serde_json::from_str(j2).unwrap();
        assert_eq!(
            c2,
            ToolCaller::ServerTool20260120 {
                tool_id: "srvtoolu_2".into()
            }
        );
    }

    #[test]
    fn unknown_caller_kind_retains_payload() {
        let j = r#"{"type":"future_caller","tool_id":"x"}"#;
        let c: ToolCaller = serde_json::from_str(j).unwrap();
        match c {
            ToolCaller::Unknown(v) => assert_eq!(v["type"], "future_caller"),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn redacted_thinking_decodes_data() {
        let j = r#"{"data":"encrypted"}"#;
        let b: RedactedThinkingBlock = serde_json::from_str(j).unwrap();
        assert_eq!(b.data, "encrypted");
    }

    #[test]
    fn server_tool_use_decodes_outer_fields_and_keeps_input_as_value() {
        let j = r#"{"id":"srvtoolu_1","name":"web_search","input":{"q":"rust"},"caller":{"type":"direct"}}"#;
        let b: ServerToolUseBlock = serde_json::from_str(j).unwrap();
        assert_eq!(b.id, "srvtoolu_1");
        assert_eq!(b.name, ServerToolName::WebSearch);
        assert_eq!(b.input["q"], "rust");
        assert_eq!(b.caller, Some(ToolCaller::Direct));
    }

    #[test]
    fn all_seven_server_tool_names_round_trip() {
        for (variant, wire) in [
            (ServerToolName::WebSearch, "web_search"),
            (ServerToolName::WebFetch, "web_fetch"),
            (ServerToolName::CodeExecution, "code_execution"),
            (ServerToolName::BashCodeExecution, "bash_code_execution"),
            (
                ServerToolName::TextEditorCodeExecution,
                "text_editor_code_execution",
            ),
            (
                ServerToolName::ToolSearchToolRegex,
                "tool_search_tool_regex",
            ),
            (ServerToolName::ToolSearchToolBm25, "tool_search_tool_bm25"),
        ] {
            assert_eq!(serde_json::to_value(variant).unwrap(), wire);
        }
    }

    #[test]
    fn container_upload_decodes_file_id() {
        let j = r#"{"file_id":"file_1"}"#;
        let b: ContainerUploadBlock = serde_json::from_str(j).unwrap();
        assert_eq!(b.file_id, "file_1");
    }

    #[test]
    fn web_search_result_carries_tool_use_id_content_and_caller() {
        let j = r#"{"tool_use_id":"toolu_1","content":[{"type":"web_search_result"}],"caller":{"type":"direct"}}"#;
        let b: WebSearchToolResultBlock = serde_json::from_str(j).unwrap();
        assert_eq!(b.tool_use_id, "toolu_1");
        assert_eq!(b.content[0]["type"], "web_search_result");
        assert_eq!(b.caller, Some(ToolCaller::Direct));
    }

    #[test]
    fn code_execution_result_carries_tool_use_id_and_content_only() {
        let j = r#"{"tool_use_id":"toolu_2","content":{"stdout":"ok"}}"#;
        let b: CodeExecutionToolResultBlock = serde_json::from_str(j).unwrap();
        assert_eq!(b.tool_use_id, "toolu_2");
        assert_eq!(b.content["stdout"], "ok");
    }
}
