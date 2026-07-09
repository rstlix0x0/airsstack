//! In-process MCP tools: the [`Tool`] seam, its result/annotation types, and
//! the ergonomic [`tool`] closure adapter.
//!
//! A [`Tool`] runs in the SDK's own process when the model invokes it. Simple
//! tools are built from a closure via [`tool`]; richer ones implement [`Tool`]
//! directly. Results carry a list of [`ToolContent`] blocks and an error flag,
//! serialized to the MCP `tools/call` result shape.

use std::future::Future;

use async_trait::async_trait;

use crate::agent::error::AgentError;

/// A single content block in a tool result. `#[non_exhaustive]`: `image` and
/// `resource` blocks are non-breaking future additions.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolContent {
    /// A text block.
    Text {
        /// The text payload.
        text: String,
    },
}

impl ToolContent {
    /// The MCP wire object for this block, e.g. `{"type":"text","text":"…"}`.
    fn to_wire(&self) -> serde_json::Value {
        match self {
            Self::Text { text } => serde_json::json!({ "type": "text", "text": text }),
        }
    }
}

/// The outcome of a tool call: content blocks plus an error flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolResult {
    /// The content blocks returned to the model.
    pub content: Vec<ToolContent>,
    /// Whether this result represents a tool-level error.
    pub is_error: bool,
}

impl ToolResult {
    /// A successful text result.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: text.into() }],
            is_error: false,
        }
    }

    /// An error text result (`isError: true`).
    #[must_use]
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: text.into() }],
            is_error: true,
        }
    }

    /// The MCP `tools/call` result object: `{content:[…], isError:bool}`.
    pub(crate) fn to_wire(&self) -> serde_json::Value {
        serde_json::json!({
            "content": self.content.iter().map(ToolContent::to_wire).collect::<Vec<_>>(),
            "isError": self.is_error,
        })
    }
}

/// MCP tool hint annotations. All optional; unset fields are omitted on the
/// wire. These are declarative wire flags, not semantic control parameters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolAnnotations {
    /// The tool does not modify its environment.
    pub read_only_hint: Option<bool>,
    /// The tool may perform destructive updates.
    pub destructive_hint: Option<bool>,
    /// Repeated calls with the same args have no additional effect.
    pub idempotent_hint: Option<bool>,
    /// The tool interacts with an open, unbounded world (e.g. the internet).
    pub open_world_hint: Option<bool>,
}

impl ToolAnnotations {
    /// The camelCase MCP wire object, omitting unset fields. Returns `None`
    /// when no hint is set so callers can skip the field entirely.
    pub(crate) fn to_wire(&self) -> Option<serde_json::Value> {
        let mut map = serde_json::Map::new();
        if let Some(v) = self.read_only_hint {
            map.insert("readOnlyHint".to_string(), v.into());
        }
        if let Some(v) = self.destructive_hint {
            map.insert("destructiveHint".to_string(), v.into());
        }
        if let Some(v) = self.idempotent_hint {
            map.insert("idempotentHint".to_string(), v.into());
        }
        if let Some(v) = self.open_world_hint {
            map.insert("openWorldHint".to_string(), v.into());
        }
        if map.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(map))
        }
    }
}

/// An in-process tool the model can call.
///
/// Implement directly for full control, or build one from a closure with
/// [`tool`]. The registry stores handlers as `Arc<dyn Tool>`.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The tool's unique name within its server.
    fn name(&self) -> &str;
    /// A human-readable description shown to the model.
    fn description(&self) -> &str;
    /// The JSON Schema for the tool's `arguments` object.
    fn input_schema(&self) -> serde_json::Value;
    /// Optional MCP hint annotations.
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
    /// Run the tool.
    ///
    /// # Errors
    /// Returning `Err` yields a model-visible error result (`isError: true`);
    /// it does not fail the session.
    async fn call(&self, input: serde_json::Value) -> Result<ToolResult, AgentError>;
}

/// A [`Tool`] built from a closure. Construct with [`tool`].
pub struct FnTool<F> {
    name: String,
    description: String,
    input_schema: serde_json::Value,
    annotations: Option<ToolAnnotations>,
    handler: F,
}

impl<F> FnTool<F> {
    /// Attach MCP hint annotations to this tool.
    #[must_use]
    pub const fn with_annotations(mut self, annotations: ToolAnnotations) -> Self {
        self.annotations = Some(annotations);
        self
    }
}

/// Build a [`Tool`] from a name, description, JSON Schema, and async handler.
///
/// The common path — no trait impl needed.
///
/// ```
/// use clauders::agent::{Tool, ToolResult, tool};
///
/// let add = tool(
///     "add",
///     "Add two integers",
///     serde_json::json!({
///         "type": "object",
///         "properties": { "a": {"type":"integer"}, "b": {"type":"integer"} },
///         "required": ["a", "b"]
///     }),
///     |args| async move {
///         let sum = args["a"].as_i64().unwrap_or(0) + args["b"].as_i64().unwrap_or(0);
///         Ok(ToolResult::text(sum.to_string()))
///     },
/// );
/// assert_eq!(add.name(), "add");
/// ```
pub fn tool<F, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    input_schema: serde_json::Value,
    handler: F,
) -> FnTool<F>
where
    F: Fn(serde_json::Value) -> Fut + Send + Sync,
    Fut: Future<Output = Result<ToolResult, AgentError>> + Send,
{
    FnTool {
        name: name.into(),
        description: description.into(),
        input_schema,
        annotations: None,
        handler,
    }
}

#[async_trait]
impl<F, Fut> Tool for FnTool<F>
where
    F: Fn(serde_json::Value) -> Fut + Send + Sync,
    Fut: Future<Output = Result<ToolResult, AgentError>> + Send,
{
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn input_schema(&self) -> serde_json::Value {
        self.input_schema.clone()
    }
    fn annotations(&self) -> Option<&ToolAnnotations> {
        self.annotations.as_ref()
    }
    async fn call(&self, input: serde_json::Value) -> Result<ToolResult, AgentError> {
        (self.handler)(input).await
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{Tool, ToolAnnotations, ToolContent, ToolResult, tool};

    #[test]
    fn text_result_wire_shape() {
        let wire = ToolResult::text("hi").to_wire();
        assert_eq!(wire["content"][0]["type"], "text");
        assert_eq!(wire["content"][0]["text"], "hi");
        assert_eq!(wire["isError"], false);
    }

    #[test]
    fn error_result_sets_is_error() {
        let wire = ToolResult::error("boom").to_wire();
        assert_eq!(wire["isError"], true);
        assert_eq!(wire["content"][0]["text"], "boom");
    }

    #[test]
    fn annotations_wire_uses_camelcase_and_omits_unset() {
        let a = ToolAnnotations {
            read_only_hint: Some(true),
            ..Default::default()
        };
        let wire = a.to_wire().expect("some annotations");
        assert_eq!(wire["readOnlyHint"], true);
        assert!(wire.get("destructiveHint").is_none());
    }

    #[test]
    fn empty_annotations_wire_is_none() {
        assert!(ToolAnnotations::default().to_wire().is_none());
    }

    #[tokio::test]
    async fn tool_adapter_calls_through() {
        let t = tool(
            "echo",
            "echoes x",
            serde_json::json!({"type":"object"}),
            |args| async move {
                Ok(ToolResult::text(
                    args["x"].as_str().unwrap_or("").to_string(),
                ))
            },
        );
        assert_eq!(t.name(), "echo");
        assert_eq!(t.description(), "echoes x");
        let res = t.call(serde_json::json!({"x":"yo"})).await.expect("call");
        assert_eq!(
            res.content,
            vec![ToolContent::Text {
                text: "yo".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn with_annotations_attaches() {
        let t = tool("t", "d", serde_json::json!({}), |_| async {
            Ok(ToolResult::text("ok"))
        })
        .with_annotations(ToolAnnotations {
            read_only_hint: Some(true),
            ..Default::default()
        });
        assert_eq!(t.annotations().expect("some").read_only_hint, Some(true));
    }
}
