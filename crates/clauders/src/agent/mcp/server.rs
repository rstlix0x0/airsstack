//! The in-process MCP server: a named, versioned set of tools.

use std::collections::HashMap;
use std::sync::Arc;

use super::tool::Tool;

/// Default server version when the builder leaves it unset.
const DEFAULT_VERSION: &str = "1.0.0";

/// A named, versioned set of in-process [`Tool`]s.
///
/// Cloning is cheap: tools are `Arc`-shared.
///
/// ```
/// use clauders::agent::{SdkMcpServer, ToolResult, tool};
///
/// let ping = tool("ping", "ping", serde_json::json!({"type":"object"}), |_| async {
///     Ok(ToolResult::text("pong"))
/// });
/// let server = SdkMcpServer::builder("demo").version("2.0.0").tool(ping).build();
/// assert_eq!(server.name(), "demo");
/// assert_eq!(server.version(), "2.0.0");
/// ```
#[derive(Clone)]
pub struct SdkMcpServer {
    name: String,
    version: String,
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl SdkMcpServer {
    /// Start building a server named `name`.
    #[must_use]
    pub fn builder(name: impl Into<String>) -> SdkMcpServerBuilder {
        SdkMcpServerBuilder {
            name: name.into(),
            version: None,
            tools: HashMap::new(),
        }
    }

    /// The server name (matches `server_name` on inbound requests).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The server version reported in the `initialize` response.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Look up a tool by name.
    pub(crate) fn tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(Arc::clone)
    }

    /// Iterate the registered tools (for `tools/list`).
    pub(crate) fn tools(&self) -> impl Iterator<Item = &Arc<dyn Tool>> {
        self.tools.values()
    }
}

/// Builder for [`SdkMcpServer`].
pub struct SdkMcpServerBuilder {
    name: String,
    version: Option<String>,
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl SdkMcpServerBuilder {
    /// Override the server version (default `"1.0.0"`).
    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Register a tool. A later tool with the same name replaces an earlier one.
    #[must_use]
    pub fn tool<T: Tool + 'static>(mut self, tool: T) -> Self {
        let handle: Arc<dyn Tool> = Arc::new(tool);
        self.tools.insert(handle.name().to_string(), handle);
        self
    }

    /// Finalize into an [`SdkMcpServer`].
    #[must_use]
    pub fn build(self) -> SdkMcpServer {
        SdkMcpServer {
            name: self.name,
            version: self.version.unwrap_or_else(|| DEFAULT_VERSION.to_string()),
            tools: self.tools,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SdkMcpServer;
    use crate::agent::mcp::tool::{Tool, ToolResult, tool};

    fn sample(name: &'static str) -> impl Tool {
        tool(name, "d", serde_json::json!({"type":"object"}), |_| async {
            Ok(ToolResult::text("ok"))
        })
    }

    #[test]
    fn builder_defaults_version() {
        let s = SdkMcpServer::builder("s").build();
        assert_eq!(s.name(), "s");
        assert_eq!(s.version(), "1.0.0");
    }

    #[test]
    fn builder_registers_and_looks_up_tools() {
        let s = SdkMcpServer::builder("s")
            .tool(sample("a"))
            .tool(sample("b"))
            .build();
        assert!(s.tool("a").is_some());
        assert!(s.tool("b").is_some());
        assert!(s.tool("c").is_none());
        assert_eq!(s.tools().count(), 2);
    }

    #[test]
    fn duplicate_tool_name_last_wins() {
        let s = SdkMcpServer::builder("s")
            .tool(sample("dup"))
            .tool(sample("dup"))
            .build();
        assert_eq!(s.tools().count(), 1);
    }
}
