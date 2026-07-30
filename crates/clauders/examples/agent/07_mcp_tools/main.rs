//! In-process MCP tools: give the agent Rust functions to call.
//!
//! An `SdkMcpServer` is a named set of tools that run inside *this* process —
//! no separate server process, no transport to configure. The SDK declares the
//! server to the binary, the model calls a tool by name, and the closure below
//! runs here and returns the result.
//!
//! The model sees these under the MCP naming convention
//! `mcp__<server>__<tool>`, which is also how they are named in an allowlist.
//!
//! This example also registers an `ElicitationPolicy`. Elicitation is how an
//! MCP server asks the *user* for structured input in the middle of a tool
//! call. The in-process tools here never elicit, so the policy shows the shape
//! rather than firing — an external MCP server that does elicit routes here.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_07_mcp_tools
//! ```

use std::sync::Arc;

use clauders::agent::cancel::CancelSignal;
use clauders::agent::error::AgentError;
use clauders::agent::{
    ContentBlock, ElicitationMode, ElicitationPolicy, ElicitationRequest, ElicitationResponse,
    Message, Options, PermissionMode, SdkMcpServer, ToolAnnotations, ToolResult, query, tool,
};
use futures_util::StreamExt;

/// Answers elicitations from a fixed policy rather than a real prompt.
struct AutoAnswer;

#[async_trait::async_trait]
impl ElicitationPolicy for AutoAnswer {
    async fn elicit(
        &self,
        request: ElicitationRequest,
        _cancel: CancelSignal,
    ) -> Result<ElicitationResponse, AgentError> {
        println!(
            "[elicitation] from {}: {}",
            request.mcp_server_name, request.message
        );
        match request.mode {
            // Form mode carries a JSON Schema the answer must satisfy. A real
            // host would render it; here we decline anything we cannot fill.
            Some(ElicitationMode::Form) => {
                println!("[elicitation] schema: {:?}", request.requested_schema);
                Ok(ElicitationResponse::Accept(
                    serde_json::json!({ "confirmed": true }),
                ))
            }
            // Url mode wants the user to visit a link (an OAuth flow, usually).
            Some(ElicitationMode::Url) => {
                println!("[elicitation] visit: {:?}", request.url);
                Ok(ElicitationResponse::Decline)
            }
            // A mode this release does not model, or none stated: decline
            // rather than guess.
            _ => Ok(ElicitationResponse::Cancel),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A tool from a closure: name, description, JSON Schema for its arguments,
    // async handler. The description is what the model reads to decide whether
    // to call it, so it is worth writing well.
    let word_count = tool(
        "word_count",
        "Count the words in a piece of text.",
        serde_json::json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        }),
        |args| async move {
            let text = args
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            Ok(ToolResult::text(
                text.split_whitespace().count().to_string(),
            ))
        },
    )
    // Optional MCP hints. Purely declarative — they tell the model what kind
    // of operation this is.
    .with_annotations(ToolAnnotations {
        read_only_hint: Some(true),
        idempotent_hint: Some(true),
        ..ToolAnnotations::default()
    });

    // Returning `ToolResult::error` reports a tool-level failure to the model
    // without failing the session; the model can read the message and retry.
    let divide = tool(
        "divide",
        "Divide one number by another.",
        serde_json::json!({
            "type": "object",
            "properties": { "a": { "type": "number" }, "b": { "type": "number" } },
            "required": ["a", "b"]
        }),
        |args| async move {
            let a = args.get("a").and_then(serde_json::Value::as_f64);
            let b = args.get("b").and_then(serde_json::Value::as_f64);
            match (a, b) {
                (Some(numerator), Some(divisor)) => {
                    if divisor.abs() < f64::EPSILON {
                        Ok(ToolResult::error("cannot divide by zero"))
                    } else {
                        Ok(ToolResult::text((numerator / divisor).to_string()))
                    }
                }
                _ => Ok(ToolResult::error("both `a` and `b` must be numbers")),
            }
        },
    );

    let server = SdkMcpServer::builder("calc")
        .version("1.0.0")
        .tool(word_count)
        .tool(divide)
        .build();

    let options = Options::builder()
        .system_prompt("Use the provided tools for arithmetic and counting. Never guess.")
        .permission_mode(PermissionMode::BypassPermissions)
        // Tools from an in-process server are named `mcp__<server>__<tool>`.
        .allowed_tools(vec![
            "mcp__calc__word_count".to_owned(),
            "mcp__calc__divide".to_owned(),
        ])
        .sdk_mcp_server(server)
        .elicitation_policy(Arc::new(AutoAnswer))
        .max_turns(8)
        .build();

    let mut stream = query(
        "How many words are in \"the quick brown fox jumps\"? \
         Then divide that count by 2, and also try dividing it by 0.",
        options,
    )
    .await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => {
                for block in &assistant.content {
                    match block {
                        ContentBlock::Text { text } => println!("{text}"),
                        ContentBlock::ToolUse { name, input, .. } => {
                            println!("[call] {name} {input}");
                        }
                        ContentBlock::ToolResult {
                            content, is_error, ..
                        } => println!("[result] is_error={is_error} {content}"),
                        _ => {}
                    }
                }
            }
            Message::Result(result) => {
                println!("---");
                println!("{}", result.result);
            }
            _ => {}
        }
    }

    Ok(())
}
