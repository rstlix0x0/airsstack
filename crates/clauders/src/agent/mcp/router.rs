//! The in-process JSON-RPC router: answers the MCP methods the backend drives
//! against an [`SdkMcpServer`] over the control protocol.
#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use super::server::SdkMcpServer;
use super::tool::{ToolAnnotations, ToolResult};

/// MCP protocol version the in-process server reports.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Handle one inbound MCP JSON-RPC `message` against `server`, returning the
/// full JSON-RPC response value to nest under `mcp_response`.
pub(crate) async fn handle_mcp_message(
    server: &SdkMcpServer,
    message: &serde_json::Value,
) -> serde_json::Value {
    let id = message
        .get("id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let method = message.get("method").and_then(serde_json::Value::as_str);

    match method {
        Some("initialize") => ok(
            &id,
            &serde_json::json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": server.name(), "version": server.version() },
            }),
        ),
        Some("notifications/initialized") => serde_json::json!({ "jsonrpc": "2.0", "result": {} }),
        Some("tools/list") => ok(&id, &serde_json::json!({ "tools": tool_list(server) })),
        Some("tools/call") => tools_call(server, id, message.get("params")).await,
        Some(_) => err(&id, -32601, "Method not found"),
        None => err(&id, -32600, "Invalid Request"),
    }
}

/// Build the `tools` array for `tools/list`.
fn tool_list(server: &SdkMcpServer) -> Vec<serde_json::Value> {
    server
        .tools()
        .map(|t| {
            let mut obj = serde_json::json!({
                "name": t.name(),
                "description": t.description(),
                "inputSchema": t.input_schema(),
            });
            if let Some(annotations) = t.annotations().and_then(ToolAnnotations::to_wire) {
                if let Some(map) = obj.as_object_mut() {
                    map.insert("annotations".to_string(), annotations);
                }
            }
            obj
        })
        .collect()
}

/// Handle `tools/call`: look up the named tool and run it.
async fn tools_call(
    server: &SdkMcpServer,
    id: serde_json::Value,
    params: Option<&serde_json::Value>,
) -> serde_json::Value {
    let name = params
        .and_then(|p| p.get("name"))
        .and_then(serde_json::Value::as_str);
    let Some(name) = name else {
        return ok(&id, &ToolResult::error("missing tool name").to_wire());
    };
    let arguments = params
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let result = match server.tool(name) {
        Some(tool) => match tool.call(arguments).await {
            Ok(result) => result,
            Err(error) => ToolResult::error(error.to_string()),
        },
        None => ToolResult::error(format!("unknown tool: {name}")),
    };
    ok(&id, &result.to_wire())
}

/// A JSON-RPC success envelope.
fn ok(id: &serde_json::Value, result: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// A JSON-RPC error envelope.
fn err(id: &serde_json::Value, code: i32, message: &str) -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::handle_mcp_message;
    use crate::agent::error::AgentError;
    use crate::agent::mcp::server::SdkMcpServer;
    use crate::agent::mcp::tool::{ToolAnnotations, ToolResult, tool};

    fn server() -> SdkMcpServer {
        let add = tool(
            "add",
            "Add two ints",
            serde_json::json!({"type":"object","properties":{"a":{"type":"integer"},"b":{"type":"integer"}}}),
            |args| async move {
                let s = args["a"].as_i64().unwrap_or(0) + args["b"].as_i64().unwrap_or(0);
                Ok(ToolResult::text(s.to_string()))
            },
        )
        .with_annotations(ToolAnnotations {
            read_only_hint: Some(true),
            ..Default::default()
        });
        let boom = tool(
            "boom",
            "always errs",
            serde_json::json!({"type":"object"}),
            |_| async {
                Err(AgentError::Protocol {
                    detail: "kaboom".into(),
                })
            },
        );
        SdkMcpServer::builder("calc").tool(add).tool(boom).build()
    }

    #[tokio::test]
    async fn initialize_reports_tools_capability_and_server_info() {
        let msg = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize"});
        let r = handle_mcp_message(&server(), &msg).await;
        assert_eq!(r["id"], 1);
        assert_eq!(r["result"]["protocolVersion"], "2024-11-05");
        assert!(r["result"]["capabilities"]["tools"].is_object());
        assert_eq!(r["result"]["serverInfo"]["name"], "calc");
    }

    #[tokio::test]
    async fn notifications_initialized_is_acked() {
        let msg = serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        let r = handle_mcp_message(&server(), &msg).await;
        assert_eq!(r["result"], serde_json::json!({}));
    }

    #[tokio::test]
    async fn tools_list_enumerates_with_schema_and_annotations() {
        let msg = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"});
        let r = handle_mcp_message(&server(), &msg).await;
        let tools = r["result"]["tools"].as_array().expect("array");
        assert_eq!(tools.len(), 2);
        let add = tools.iter().find(|t| t["name"] == "add").expect("add tool");
        assert_eq!(add["description"], "Add two ints");
        assert_eq!(add["inputSchema"]["type"], "object");
        assert_eq!(add["annotations"]["readOnlyHint"], true);
    }

    #[tokio::test]
    async fn tools_call_runs_handler() {
        let msg = serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"add","arguments":{"a":2,"b":3}}});
        let r = handle_mcp_message(&server(), &msg).await;
        assert_eq!(r["id"], 3);
        assert_eq!(r["result"]["content"][0]["text"], "5");
        assert_eq!(r["result"]["isError"], false);
    }

    #[tokio::test]
    async fn tools_call_handler_error_is_model_visible() {
        let msg = serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"boom","arguments":{}}});
        let r = handle_mcp_message(&server(), &msg).await;
        assert_eq!(r["result"]["isError"], true);
        assert!(
            r["result"]["content"][0]["text"]
                .as_str()
                .expect("text")
                .contains("kaboom")
        );
    }

    #[tokio::test]
    async fn tools_call_unknown_tool_is_error_result() {
        let msg = serde_json::json!({"jsonrpc":"2.0","id":5,"method":"tools/call",
            "params":{"name":"nope","arguments":{}}});
        let r = handle_mcp_message(&server(), &msg).await;
        assert_eq!(r["result"]["isError"], true);
        assert!(
            r["result"]["content"][0]["text"]
                .as_str()
                .expect("text")
                .contains("unknown tool")
        );
    }

    #[tokio::test]
    async fn unknown_method_is_minus_32601() {
        let msg = serde_json::json!({"jsonrpc":"2.0","id":6,"method":"resources/list"});
        let r = handle_mcp_message(&server(), &msg).await;
        assert_eq!(r["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn missing_method_is_minus_32600() {
        let msg = serde_json::json!({"jsonrpc":"2.0","id":7});
        let r = handle_mcp_message(&server(), &msg).await;
        assert_eq!(r["error"]["code"], -32600);
    }
}
