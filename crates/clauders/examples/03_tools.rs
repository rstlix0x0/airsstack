//! Tool (function calling) round-trip.
//!
//! Sends a request with a weather tool, reads the tool-use block from the
//! assistant turn, and sends the result back in a follow-up message.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 03_tools --features messages-tools,transport-reqwest
//! ```

use clauders::messages::tools::{Tool, ToolChoice, ToolResultBlock};
use clauders::messages::{ContentBlock, MessageContent, Role};
use clauders::prelude::*;
use clauders::types::ToolName;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(1024)?;
    let tool_name = ToolName::new("get_weather")?;

    let tool = Tool {
        name: tool_name,
        description: "Look up the current weather for a city.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"]
        }),
        #[cfg(feature = "messages-caching")]
        cache_control: None,
        #[cfg(all(feature = "messages-tools", feature = "messages-structured-outputs"))]
        strict: None,
    };

    // First turn: ask a question that triggers a tool call.
    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(max_tokens)
        .add_user_text("What is the weather in Paris?")
        .tools([tool.clone()])
        .tool_choice(ToolChoice::Auto)
        .build();

    let assistant_msg = client.messages().create(req).await?;
    println!(
        "First response stop_reason: {:?}",
        assistant_msg.stop_reason
    );

    // Find the tool-use block.
    let tool_use = assistant_msg.content.iter().find_map(|b| {
        if let ContentBlock::ToolUse(tu) = b {
            Some(tu.clone())
        } else {
            None
        }
    });

    let Some(tu) = tool_use else {
        println!("Model did not call the tool.");
        return Ok(());
    };

    println!("Tool called: {} with input: {}", tu.name.as_str(), tu.input);

    // Build the tool result.
    let result_body =
        serde_json::json!({"temperature": "18°C", "condition": "partly cloudy"}).to_string();
    let tool_result = ToolResultBlock::text(tu.id.clone(), result_body);

    // Second turn: send the tool result back.
    let follow_up = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(max_tokens)
        .add_message(
            Role::User,
            MessageContent::Text("What is the weather in Paris?".into()),
        )
        .add_message(
            Role::Assistant,
            MessageContent::Blocks(assistant_msg.content.clone()),
        )
        .add_message(
            Role::User,
            MessageContent::Blocks(vec![ContentBlock::ToolResult(tool_result)]),
        )
        .tools([tool])
        .tool_choice(ToolChoice::Auto)
        .build();

    let final_msg = client.messages().create(follow_up).await?;

    for block in &final_msg.content {
        if let ContentBlock::Text(t) = block {
            println!("{}", t.text);
        }
    }

    Ok(())
}
