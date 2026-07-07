//! Bridge the in-process MCP registry to the Messages API tool surface:
//! declare every registered tool as a namespaced wire `Tool`, and dispatch a
//! model tool call back to the owning `SdkMcpServer`.

use crate::agent::mcp::SdkMcpRegistry;
use crate::agent::mcp::tool::{ToolContent, ToolResult};
use crate::messages::tools::{Tool as WireTool, ToolResultBlock, ToolUseBlock};
use crate::types::ToolName;

use super::convert::{declare_name, route};

/// Declare every registered in-process tool as a namespaced Messages API tool.
///
/// Tools whose namespaced name is empty (impossible for non-empty server/tool
/// names) are skipped; in practice every registered tool is declared.
pub(super) fn declare(registry: &SdkMcpRegistry) -> Vec<WireTool> {
    let mut tools = Vec::new();
    for server in registry.servers() {
        for tool in server.tools() {
            let name = declare_name(server.name(), tool.name());
            let Ok(name) = ToolName::new(name) else {
                continue;
            };
            tools.push(WireTool {
                name,
                description: tool.description().to_string(),
                input_schema: tool.input_schema(),
                cache_control: None,
                strict: None,
            });
        }
    }
    tools
}

/// Route a model tool call to its owning server, run it, and shape the result
/// as a wire `ToolResultBlock`. A handler error, an unknown server/tool, or an
/// unroutable name all become a model-visible error result — never a session
/// failure, matching the JSON-RPC router's contract.
pub(super) async fn dispatch(registry: &SdkMcpRegistry, block: &ToolUseBlock) -> ToolResultBlock {
    let id = block.id.clone();
    let Ok((server_name, tool_name)) = route(block.name.as_str()) else {
        return ToolResultBlock::err(id, format!("unroutable tool name: {}", block.name.as_str()));
    };
    let Some(server) = registry.lookup(server_name) else {
        return ToolResultBlock::err(id, format!("unknown server: {server_name}"));
    };
    let Some(tool) = server.tool(tool_name) else {
        return ToolResultBlock::err(id, format!("unknown tool: {tool_name}"));
    };
    match tool.call(block.input.clone()).await {
        Ok(result) => to_result_block(id, &result),
        Err(error) => ToolResultBlock::err(id, error.to_string()),
    }
}

/// Convert an in-process `ToolResult` to a wire `ToolResultBlock`, preserving
/// the error flag and flattening text content.
fn to_result_block(id: crate::types::ToolUseId, result: &ToolResult) -> ToolResultBlock {
    let text: String = result
        .content
        .iter()
        .map(|c| match c {
            ToolContent::Text { text } => text.as_str(),
        })
        .collect();
    if result.is_error {
        ToolResultBlock::err(id, text)
    } else {
        ToolResultBlock::text(id, text)
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{declare, dispatch};
    use crate::agent::mcp::tool::{ToolResult, tool};
    use crate::agent::mcp::{SdkMcpRegistry, SdkMcpServer};
    use crate::messages::tools::{ToolResultContent, ToolUseBlock};
    use crate::types::{ToolName, ToolUseId};

    fn registry() -> SdkMcpRegistry {
        let add = tool(
            "add",
            "Add two ints",
            serde_json::json!({"type": "object"}),
            |args| async move {
                let s = args["a"].as_i64().unwrap_or(0) + args["b"].as_i64().unwrap_or(0);
                Ok(ToolResult::text(s.to_string()))
            },
        );
        let mut reg = SdkMcpRegistry::default();
        reg.register(SdkMcpServer::builder("calc").tool(add).build());
        reg
    }

    fn tool_use(name: &str, input: serde_json::Value) -> ToolUseBlock {
        ToolUseBlock {
            id: ToolUseId::new("toolu_1").expect("id"),
            name: ToolName::new(name).expect("name"),
            input,
            cache_control: None,
        }
    }

    #[test]
    fn declare_namespaces_every_registered_tool() {
        let tools = declare(&registry());
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name.as_str(), "mcp__calc__add");
        assert_eq!(tools[0].description, "Add two ints");
    }

    #[tokio::test]
    async fn dispatch_runs_the_owning_tool() {
        let block = tool_use("mcp__calc__add", serde_json::json!({"a": 2, "b": 3}));
        let result = dispatch(&registry(), &block).await;
        assert_eq!(result.tool_use_id.as_str(), "toolu_1");
        assert!(result.is_error.is_none());
        assert_eq!(result.content, ToolResultContent::Text("5".into()));
    }

    #[tokio::test]
    async fn dispatch_unknown_server_is_an_error_result() {
        let block = tool_use("mcp__nope__add", serde_json::json!({}));
        let result = dispatch(&registry(), &block).await;
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn dispatch_unroutable_name_is_an_error_result() {
        let block = tool_use("bash", serde_json::json!({}));
        let result = dispatch(&registry(), &block).await;
        assert_eq!(result.is_error, Some(true));
    }
}
