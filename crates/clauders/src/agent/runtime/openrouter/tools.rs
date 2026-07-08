//! Bridge the in-process MCP registry to the OpenRouter function-tool surface:
//! declare every registered tool as a namespaced function tool, and dispatch a
//! model tool call back to the owning `SdkMcpServer` as a `tool`-role message.

use openrouter_rs::chat::message::Message as OrMessage;
use openrouter_rs::chat::tool::{FunctionDef, Tool as OrTool};
use openrouter_rs::chat::tool_call::ToolCall;
use openrouter_rs::types::FunctionName;

use crate::agent::mcp::SdkMcpRegistry;
use crate::agent::mcp::naming::{declare_name, route};
use crate::agent::mcp::tool::{ToolContent, ToolResult};

/// Declare every registered in-process tool as a namespaced OpenRouter function
/// tool. A tool whose namespaced name is not a valid `FunctionName` is skipped
/// (impossible for non-empty server/tool names in practice).
pub(super) fn declare(registry: &SdkMcpRegistry) -> Vec<OrTool> {
    let mut tools = Vec::new();
    for server in registry.servers() {
        for tool in server.tools() {
            let Ok(name) = FunctionName::new(declare_name(server.name(), tool.name())) else {
                continue;
            };
            tools.push(OrTool::function(FunctionDef {
                name,
                description: Some(tool.description().to_string()),
                parameters: Some(tool.input_schema()),
                strict: None,
            }));
        }
    }
    tools
}

/// Route a model tool call to its owning server, run it, and shape the result as
/// a `tool`-role message keyed on the call id. A bad-arguments parse, an
/// unroutable name, an unknown server/tool, or a handler error all become a
/// model-visible error message — never a session failure.
pub(super) async fn dispatch(registry: &SdkMcpRegistry, call: &ToolCall) -> OrMessage {
    let id = call.id.clone();
    let input: serde_json::Value = match serde_json::from_str(&call.function.arguments) {
        Ok(value) => value,
        Err(error) => {
            return OrMessage::tool_result(id, format!("invalid tool arguments: {error}"));
        }
    };
    let Ok((server_name, tool_name)) = route(&call.function.name) else {
        return OrMessage::tool_result(id, format!("unroutable tool name: {}", call.function.name));
    };
    let Some(server) = registry.lookup(server_name) else {
        return OrMessage::tool_result(id, format!("unknown server: {server_name}"));
    };
    let Some(tool) = server.tool(tool_name) else {
        return OrMessage::tool_result(id, format!("unknown tool: {tool_name}"));
    };
    let text = match tool.call(input).await {
        Ok(result) => flatten(&result),
        Err(error) => error.to_string(),
    };
    OrMessage::tool_result(id, text)
}

/// Flatten an in-process `ToolResult` to its text payload.
fn flatten(result: &ToolResult) -> String {
    result
        .content
        .iter()
        .map(|c| match c {
            ToolContent::Text { text } => text.as_str(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{declare, dispatch};
    use crate::agent::mcp::tool::{ToolResult, tool};
    use crate::agent::mcp::{SdkMcpRegistry, SdkMcpServer};
    use openrouter_rs::chat::message::{MessageContent, Role};
    use openrouter_rs::chat::tool::ToolType;
    use openrouter_rs::chat::tool_call::{FunctionCall, ToolCall};
    use openrouter_rs::types::ToolCallId;

    fn registry() -> SdkMcpRegistry {
        let add = tool(
            "add",
            "Add two ints",
            serde_json::json!({ "type": "object" }),
            |args| async move {
                let s = args["a"].as_i64().unwrap_or(0) + args["b"].as_i64().unwrap_or(0);
                Ok(ToolResult::text(s.to_string()))
            },
        );
        let mut reg = SdkMcpRegistry::default();
        reg.register(SdkMcpServer::builder("calc").tool(add).build());
        reg
    }

    fn call(name: &str, args: &str) -> ToolCall {
        ToolCall {
            id: ToolCallId::new("call_1").expect("id"),
            r#type: ToolType::Function,
            function: FunctionCall {
                name: name.into(),
                arguments: args.into(),
            },
        }
    }

    #[test]
    fn declare_namespaces_every_registered_tool() {
        let tools = declare(&registry());
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name.as_str(), "mcp__calc__add");
        assert_eq!(
            tools[0].function.description.as_deref(),
            Some("Add two ints")
        );
    }

    #[tokio::test]
    async fn dispatch_runs_the_owning_tool() {
        let msg = dispatch(&registry(), &call("mcp__calc__add", r#"{"a":2,"b":3}"#)).await;
        assert_eq!(msg.role(), Role::Tool);
        assert_eq!(msg.content(), Some(&MessageContent::Text("5".into())));
    }

    #[tokio::test]
    async fn dispatch_unknown_server_is_an_error_message() {
        let msg = dispatch(&registry(), &call("mcp__nope__add", "{}")).await;
        assert_eq!(msg.role(), Role::Tool);
        // The error is conveyed as message text, not a session failure.
        assert!(
            matches!(msg.content(), Some(MessageContent::Text(t)) if t.contains("unknown server"))
        );
    }

    #[tokio::test]
    async fn dispatch_unroutable_name_is_an_error_message() {
        let msg = dispatch(&registry(), &call("bash", "{}")).await;
        assert!(matches!(msg.content(), Some(MessageContent::Text(t)) if t.contains("unroutable")));
    }

    #[tokio::test]
    async fn dispatch_bad_arguments_is_an_error_message() {
        let msg = dispatch(&registry(), &call("mcp__calc__add", "not-json")).await;
        assert!(
            matches!(msg.content(), Some(MessageContent::Text(t)) if t.contains("invalid tool arguments"))
        );
    }
}
