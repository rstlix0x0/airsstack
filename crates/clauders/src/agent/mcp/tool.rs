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

/// A single content block in a tool result. `#[non_exhaustive]`: the MCP content
/// set may grow, so new block kinds are non-breaking future additions.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolContent {
    /// A text block.
    Text {
        /// The text payload.
        text: String,
    },
    /// An image block. `data` is base64-encoded image bytes.
    Image {
        /// Base64-encoded image data.
        data: String,
        /// The IANA media type, e.g. `image/png`.
        mime_type: String,
    },
    /// An audio block. `data` is base64-encoded audio bytes.
    Audio {
        /// Base64-encoded audio data.
        data: String,
        /// The IANA media type, e.g. `audio/wav`.
        mime_type: String,
    },
    /// A link to a resource the client may fetch or subscribe to.
    ResourceLink {
        /// The resource URI.
        uri: String,
        /// A short human-readable name.
        name: String,
        /// An optional human-readable description.
        description: Option<String>,
        /// The optional IANA media type of the linked resource.
        mime_type: Option<String>,
    },
    /// An embedded resource carrying its content inline.
    Resource {
        /// The embedded resource body (text or binary).
        resource: ResourceContents,
    },
}

/// The body of an embedded [`ToolContent::Resource`]: text XOR binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceContents {
    /// A text resource.
    Text {
        /// The resource URI.
        uri: String,
        /// The optional IANA media type.
        mime_type: Option<String>,
        /// The text payload.
        text: String,
    },
    /// A binary resource. `blob` is base64-encoded.
    Blob {
        /// The resource URI.
        uri: String,
        /// The optional IANA media type.
        mime_type: Option<String>,
        /// Base64-encoded binary payload.
        blob: String,
    },
}

impl ResourceContents {
    /// The inner MCP `resource` object for an embedded resource block.
    fn to_wire(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        match self {
            Self::Text {
                uri,
                mime_type,
                text,
            } => {
                map.insert("uri".to_string(), uri.clone().into());
                if let Some(m) = mime_type {
                    map.insert("mimeType".to_string(), m.clone().into());
                }
                map.insert("text".to_string(), text.clone().into());
            }
            Self::Blob {
                uri,
                mime_type,
                blob,
            } => {
                map.insert("uri".to_string(), uri.clone().into());
                if let Some(m) = mime_type {
                    map.insert("mimeType".to_string(), m.clone().into());
                }
                map.insert("blob".to_string(), blob.clone().into());
            }
        }
        serde_json::Value::Object(map)
    }
}

impl ToolContent {
    /// The MCP wire object for this block, e.g. `{"type":"text","text":"…"}`.
    fn to_wire(&self) -> serde_json::Value {
        match self {
            Self::Text { text } => serde_json::json!({ "type": "text", "text": text }),
            Self::Image { data, mime_type } => {
                serde_json::json!({ "type": "image", "data": data, "mimeType": mime_type })
            }
            Self::Audio { data, mime_type } => {
                serde_json::json!({ "type": "audio", "data": data, "mimeType": mime_type })
            }
            Self::ResourceLink {
                uri,
                name,
                description,
                mime_type,
            } => {
                let mut map = serde_json::Map::new();
                map.insert("type".to_string(), "resource_link".into());
                map.insert("uri".to_string(), uri.clone().into());
                map.insert("name".to_string(), name.clone().into());
                if let Some(d) = description {
                    map.insert("description".to_string(), d.clone().into());
                }
                if let Some(m) = mime_type {
                    map.insert("mimeType".to_string(), m.clone().into());
                }
                serde_json::Value::Object(map)
            }
            Self::Resource { resource } => {
                serde_json::json!({ "type": "resource", "resource": resource.to_wire() })
            }
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

    /// A successful image result. `data` is base64-encoded.
    ///
    /// ```
    /// use clauders::agent::ToolResult;
    /// let r = ToolResult::image("<base64>", "image/png");
    /// assert!(!r.is_error);
    /// ```
    #[must_use]
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Image {
                data: data.into(),
                mime_type: mime_type.into(),
            }],
            is_error: false,
        }
    }

    /// A successful audio result. `data` is base64-encoded.
    ///
    /// ```
    /// use clauders::agent::ToolResult;
    /// let r = ToolResult::audio("<base64>", "audio/wav");
    /// assert!(!r.is_error);
    /// ```
    #[must_use]
    pub fn audio(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Audio {
                data: data.into(),
                mime_type: mime_type.into(),
            }],
            is_error: false,
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

    use super::{ResourceContents, Tool, ToolAnnotations, ToolContent, ToolResult, tool};

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

    #[test]
    fn image_wire_shape() {
        let wire = ToolResult::image("BASE64", "image/png").to_wire();
        assert_eq!(wire["content"][0]["type"], "image");
        assert_eq!(wire["content"][0]["data"], "BASE64");
        assert_eq!(wire["content"][0]["mimeType"], "image/png");
        assert_eq!(wire["isError"], false);
    }

    #[test]
    fn audio_wire_shape() {
        let wire = ToolResult::audio("AUDIO64", "audio/wav").to_wire();
        assert_eq!(wire["content"][0]["type"], "audio");
        assert_eq!(wire["content"][0]["data"], "AUDIO64");
        assert_eq!(wire["content"][0]["mimeType"], "audio/wav");
    }

    #[test]
    fn image_and_audio_constructors_are_single_block_success() {
        let img = ToolResult::image("d", "image/png");
        assert!(!img.is_error);
        assert_eq!(
            img.content,
            vec![ToolContent::Image {
                data: "d".to_string(),
                mime_type: "image/png".to_string(),
            }]
        );
        assert!(!ToolResult::audio("d", "audio/wav").is_error);
    }

    #[test]
    fn resource_link_wire_shape_omits_unset() {
        let bare = ToolResult {
            content: vec![ToolContent::ResourceLink {
                uri: "file:///m.rs".to_string(),
                name: "m.rs".to_string(),
                description: None,
                mime_type: None,
            }],
            is_error: false,
        };
        let block = &bare.to_wire()["content"][0];
        assert_eq!(block["type"], "resource_link");
        assert_eq!(block["uri"], "file:///m.rs");
        assert_eq!(block["name"], "m.rs");
        assert!(block.get("description").is_none());
        assert!(block.get("mimeType").is_none());

        let full = ToolResult {
            content: vec![ToolContent::ResourceLink {
                uri: "file:///m.rs".to_string(),
                name: "m.rs".to_string(),
                description: Some("entry".to_string()),
                mime_type: Some("text/x-rust".to_string()),
            }],
            is_error: false,
        };
        let block = &full.to_wire()["content"][0];
        assert_eq!(block["description"], "entry");
        assert_eq!(block["mimeType"], "text/x-rust");
    }

    #[test]
    fn embedded_resource_text_wire_shape() {
        let r = ToolResult {
            content: vec![ToolContent::Resource {
                resource: ResourceContents::Text {
                    uri: "file:///m.rs".to_string(),
                    mime_type: Some("text/x-rust".to_string()),
                    text: "fn main() {}".to_string(),
                },
            }],
            is_error: false,
        };
        let block = &r.to_wire()["content"][0];
        assert_eq!(block["type"], "resource");
        assert_eq!(block["resource"]["uri"], "file:///m.rs");
        assert_eq!(block["resource"]["text"], "fn main() {}");
        assert_eq!(block["resource"]["mimeType"], "text/x-rust");
        assert!(block["resource"].get("blob").is_none());
    }

    #[test]
    fn embedded_resource_blob_wire_shape() {
        let r = ToolResult {
            content: vec![ToolContent::Resource {
                resource: ResourceContents::Blob {
                    uri: "file:///img.png".to_string(),
                    mime_type: None,
                    blob: "BLOB64".to_string(),
                },
            }],
            is_error: false,
        };
        let block = &r.to_wire()["content"][0];
        assert_eq!(block["resource"]["blob"], "BLOB64");
        assert!(block["resource"].get("text").is_none());
        assert!(block["resource"].get("mimeType").is_none());
    }
}
