//! Tool definitions and tool-use content blocks for the Messages API.
//!
//! Exists as its own module so tool-calling types are only compiled when
//! the `messages-tools` feature is enabled, keeping the base messages
//! surface free of tool-specific dependencies.
//!
//! Responsibilities:
//! - Define [`Tool`], the callable function description sent in a request.
//! - Define [`ToolChoice`], controlling how the model selects a tool.
//! - Define [`ToolUseBlock`] and [`ToolResultBlock`], the content-block
//!   shapes for tool invocations and their results.
//! - Define [`ToolResultContent`], the body of a tool result (text or blocks).
//!
//! Not responsible for:
//! - Registering tool blocks inside [`crate::messages::ContentBlock`] —
//!   that lives in `content.rs` under the same feature gate.
//! - HTTP transport or request sending.

use crate::messages::content::ContentBlock;
use crate::types::{ToolName, ToolUseId};

/// A callable function the model may invoke during a generation turn.
///
/// Construct directly and pass via [`crate::messages::MessageRequestBuilder::tools`].
///
/// # Examples
///
/// ```
/// use clauders::messages::tools::Tool;
/// use clauders::types::ToolName;
///
/// let tool = Tool {
///     name: ToolName::new("get_weather").unwrap(),
///     description: "Retrieve current weather for a city.".into(),
///     input_schema: serde_json::json!({
///         "type": "object",
///         "properties": { "city": { "type": "string" } },
///         "required": ["city"]
///     }),
///     #[cfg(feature = "messages-caching")]
///     cache_control: None,
///     #[cfg(all(feature = "messages-tools", feature = "messages-structured-outputs"))]
///     strict: None,
/// };
/// let j = serde_json::to_value(&tool).unwrap();
/// assert_eq!(j["name"], "get_weather");
/// ```
#[derive(Clone, Debug, serde::Serialize)]
pub struct Tool {
    /// The name the model uses to invoke this function.
    pub name: ToolName,
    /// Human-readable description of what the function does.
    pub description: String,
    /// JSON Schema describing the function's input parameters.
    pub input_schema: serde_json::Value,
    /// Optional cache breakpoint for this tool definition.
    ///
    /// When set, this tool marks a prompt-caching boundary in the tool list.
    /// Requires the `messages-caching` feature.
    #[cfg(feature = "messages-caching")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<crate::types::CacheControl>,
    /// Constrain the model's tool input to this tool's `input_schema`.
    ///
    /// Wire-format boolean; serialized only when set.
    #[cfg(all(feature = "messages-tools", feature = "messages-structured-outputs"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Controls which tool, if any, the model must call.
///
/// # Examples
///
/// ```
/// use clauders::messages::tools::ToolChoice;
/// let j = serde_json::to_string(&ToolChoice::Auto).unwrap();
/// assert_eq!(j, r#"{"type":"auto"}"#);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Model may call any tool or none at all.
    Auto,
    /// Model must call at least one tool.
    Any,
    /// Model must call the named tool.
    Tool {
        /// Name of the tool the model is required to call.
        name: ToolName,
    },
    /// Model must not call any tool.
    None,
}

/// Content block produced when the model invokes a tool.
///
/// Appears inside [`crate::messages::ContentBlock::ToolUse`] in a response.
///
/// # Examples
///
/// ```
/// use clauders::messages::tools::ToolUseBlock;
/// use clauders::types::{ToolName, ToolUseId};
///
/// let block = ToolUseBlock {
///     id:    ToolUseId::new("toolu_01").unwrap(),
///     name:  ToolName::new("get_weather").unwrap(),
///     input: serde_json::json!({"city": "Paris"}),
///     #[cfg(feature = "messages-caching")]
///     cache_control: None,
/// };
/// assert_eq!(block.name.as_str(), "get_weather");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolUseBlock {
    /// Server-assigned identifier correlating this invocation with its result.
    pub id: ToolUseId,
    /// Name of the tool being called.
    pub name: ToolName,
    /// Arguments supplied by the model, matching the tool's `input_schema`.
    pub input: serde_json::Value,
    /// Optional cache breakpoint for this tool-use block.
    ///
    /// Requires the `messages-caching` feature.
    #[cfg(feature = "messages-caching")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cache_control: Option<crate::types::CacheControl>,
}

/// Content block carrying the result of a tool invocation.
///
/// Appears inside [`crate::messages::ContentBlock::ToolResult`] in a
/// follow-up user turn.
///
/// Use [`ToolResultBlock::text`] for a plain-text result and
/// [`ToolResultBlock::err`] to signal a tool-execution failure.
///
/// # Examples
///
/// ```
/// use clauders::messages::tools::ToolResultBlock;
/// use clauders::types::ToolUseId;
///
/// let result = ToolResultBlock::text(
///     ToolUseId::new("toolu_01").unwrap(),
///     r#"{"temperature": 24}"#,
/// );
/// assert!(result.is_error.is_none());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolResultBlock {
    /// The [`ToolUseBlock::id`] this result is responding to.
    pub tool_use_id: ToolUseId,
    /// Body of the result.
    pub content: ToolResultContent,
    /// Set to `true` when the tool execution failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    /// Optional cache breakpoint for this tool-result block.
    ///
    /// Requires the `messages-caching` feature.
    #[cfg(feature = "messages-caching")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cache_control: Option<crate::types::CacheControl>,
}

impl ToolResultBlock {
    /// Construct a successful plain-text tool result.
    #[must_use]
    pub fn text(tool_use_id: ToolUseId, body: impl Into<String>) -> Self {
        Self {
            tool_use_id,
            content: ToolResultContent::Text(body.into()),
            is_error: None,
            #[cfg(feature = "messages-caching")]
            cache_control: None,
        }
    }

    /// Construct a tool-error result carrying a plain-text error message.
    #[must_use]
    pub fn err(tool_use_id: ToolUseId, body: impl Into<String>) -> Self {
        Self {
            tool_use_id,
            content: ToolResultContent::Text(body.into()),
            is_error: Some(true),
            #[cfg(feature = "messages-caching")]
            cache_control: None,
        }
    }
}

/// Body of a [`ToolResultBlock`]: either a plain string or typed content blocks.
///
/// The untagged representation matches the Anthropic API wire format: a JSON
/// string maps to `Text`; a JSON array maps to `Blocks`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Plain-text result.
    Text(String),
    /// Structured content blocks.
    Blocks(Vec<ContentBlock>),
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn tool_serializes_with_schema() {
        let t = Tool {
            name: ToolName::new("get_weather").unwrap(),
            description: "Look up weather".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"city":{"type":"string"}}}),
            #[cfg(feature = "messages-caching")]
            cache_control: None,
            #[cfg(all(feature = "messages-tools", feature = "messages-structured-outputs"))]
            strict: None,
        };
        let j = serde_json::to_value(&t).unwrap();
        assert_eq!(j["name"], "get_weather");
        assert_eq!(j["input_schema"]["type"], "object");
    }

    #[test]
    fn tool_choice_tagged_correctly() {
        assert_eq!(
            serde_json::to_string(&ToolChoice::Auto).unwrap(),
            r#"{"type":"auto"}"#
        );
        assert_eq!(
            serde_json::to_string(&ToolChoice::Tool {
                name: ToolName::new("x").unwrap()
            })
            .unwrap(),
            r#"{"type":"tool","name":"x"}"#
        );
    }

    #[test]
    fn tool_result_block_text_has_no_error_flag() {
        let id = ToolUseId::new("toolu_01").unwrap();
        let r = ToolResultBlock::text(id, "ok");
        assert!(r.is_error.is_none());
        assert_eq!(r.content, ToolResultContent::Text("ok".into()));
    }

    #[test]
    fn tool_result_block_err_sets_is_error_true() {
        let id = ToolUseId::new("toolu_02").unwrap();
        let r = ToolResultBlock::err(id, "boom");
        assert_eq!(r.is_error, Some(true));
    }

    #[test]
    fn tool_use_block_round_trips_via_serde() {
        let block = ToolUseBlock {
            id: ToolUseId::new("toolu_01").unwrap(),
            name: ToolName::new("get_weather").unwrap(),
            input: serde_json::json!({"city": "Paris"}),
            #[cfg(feature = "messages-caching")]
            cache_control: None,
        };
        let j = serde_json::to_string(&block).unwrap();
        let back: ToolUseBlock = serde_json::from_str(&j).unwrap();
        assert_eq!(back.id, block.id);
        assert_eq!(back.name, block.name);
        assert_eq!(back.input["city"], "Paris");
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn tool_with_cache_serializes_field() {
        use crate::types::{CacheControl, ToolName};
        let t = Tool {
            name: ToolName::new("search").unwrap(),
            description: "Search".into(),
            input_schema: serde_json::json!({"type":"object"}),
            cache_control: Some(CacheControl::ephemeral()),
            #[cfg(all(feature = "messages-tools", feature = "messages-structured-outputs"))]
            strict: None,
        };
        let j = serde_json::to_value(&t).unwrap();
        assert_eq!(j["cache_control"]["type"], "ephemeral");
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn tool_without_cache_omits_field() {
        use crate::types::ToolName;
        let t = Tool {
            name: ToolName::new("search").unwrap(),
            description: "Search".into(),
            input_schema: serde_json::json!({"type":"object"}),
            cache_control: None,
            #[cfg(all(feature = "messages-tools", feature = "messages-structured-outputs"))]
            strict: None,
        };
        let j = serde_json::to_value(&t).unwrap();
        assert!(j.get("cache_control").is_none());
    }

    #[cfg(all(feature = "messages-tools", feature = "messages-structured-outputs"))]
    #[test]
    fn tool_strict_true_serializes_field() {
        let t = Tool {
            name: ToolName::new("strict_tool").unwrap(),
            description: "A strictly-typed tool.".into(),
            input_schema: serde_json::json!({"type":"object"}),
            #[cfg(feature = "messages-caching")]
            cache_control: None,
            strict: Some(true),
        };
        let j = serde_json::to_value(&t).unwrap();
        assert_eq!(
            j["strict"], true,
            "strict: Some(true) must serialize as true"
        );
    }

    #[cfg(all(feature = "messages-tools", feature = "messages-structured-outputs"))]
    #[test]
    fn tool_strict_none_omits_field() {
        let t = Tool {
            name: ToolName::new("lax_tool").unwrap(),
            description: "A non-strict tool.".into(),
            input_schema: serde_json::json!({"type":"object"}),
            #[cfg(feature = "messages-caching")]
            cache_control: None,
            strict: None,
        };
        let j = serde_json::to_value(&t).unwrap();
        assert!(
            j.get("strict").is_none(),
            "strict: None must omit the field from the wire payload"
        );
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn tool_use_block_with_cache_round_trips() {
        use crate::types::CacheControl;
        let block = ToolUseBlock {
            id: ToolUseId::new("toolu_01").unwrap(),
            name: ToolName::new("get_weather").unwrap(),
            input: serde_json::json!({"city": "Paris"}),
            cache_control: Some(CacheControl::ephemeral()),
        };
        let j = serde_json::to_string(&block).unwrap();
        let back: ToolUseBlock = serde_json::from_str(&j).unwrap();
        assert_eq!(back.cache_control, Some(CacheControl::ephemeral()));
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn tool_result_block_text_ctor_initializes_cache_control_none() {
        let id = ToolUseId::new("toolu_01").unwrap();
        let r = ToolResultBlock::text(id, "ok");
        assert!(r.cache_control.is_none());
    }

    #[cfg(feature = "messages-caching")]
    #[test]
    fn tool_result_block_err_ctor_initializes_cache_control_none() {
        let id = ToolUseId::new("toolu_02").unwrap();
        let r = ToolResultBlock::err(id, "boom");
        assert!(r.cache_control.is_none());
    }
}
