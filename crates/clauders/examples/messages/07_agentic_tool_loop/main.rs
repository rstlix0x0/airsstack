//! Agentic tool loop — keep calling tools until the model is done.
//!
//! `03_tools` does a single tool round-trip. This drives the full loop: send
//! the conversation, and while the model stops on `tool_use`, run every tool
//! call it made, append the results, and send again — until the model returns
//! a normal end-of-turn answer. Asking about two cities forces more than one
//! tool call before the model settles.
//!
//! The tool here is faked in-process (`weather_for`); a real application would
//! call an actual weather service.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 07_agentic_tool_loop
//! ```

use clauders::messages::MessageContent;
use clauders::messages::tools::{Tool, ToolChoice, ToolResultBlock};
use clauders::prelude::*;
use clauders::types::ToolName;

/// Stand-in for a real weather lookup.
fn weather_for(input: &serde_json::Value) -> String {
    let city = input
        .get("city")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let report = match city {
        "Paris" => "18°C, partly cloudy",
        "Tokyo" => "24°C, clear",
        _ => "no data",
    };
    serde_json::json!({ "city": city, "weather": report }).to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(1024);

    let tool = Tool {
        name: ToolName::new("get_weather")?,
        description: "Look up the current weather for a city.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"]
        }),
        cache_control: None,
        strict: None,
        eager_input_streaming: None,
    };

    // The running conversation. Each turn appends the assistant reply and, when
    // it called tools, the tool results.
    let mut history: Vec<(Role, MessageContent)> = vec![(
        Role::User,
        MessageContent::Text("What is the weather in Paris and Tokyo?".into()),
    )];

    // Cap the loop so a misbehaving model cannot spin forever.
    for turn in 1..=6 {
        let mut builder = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(max_tokens);
        for (role, content) in &history {
            builder = builder.add_message(*role, content.clone());
        }
        let req = builder
            .tools([tool.clone()])
            .tool_choice(ToolChoice::Auto)
            .build();

        let msg = client.messages().create(req).await?;
        println!("--- turn {turn}: stop_reason {:?} ---", msg.stop_reason);

        // Show any text the model produced this turn.
        for block in &msg.content {
            if let ContentBlock::Text(t) = block {
                println!("{}", t.text);
            }
        }

        // Record the assistant turn so the next request carries it.
        history.push((
            Role::Assistant,
            MessageContent::Blocks(ContentBlockParam::try_from_response(msg.content.clone())?),
        ));

        // Done once the model stops for any reason other than a tool call.
        if msg.stop_reason != Some(StopReason::ToolUse) {
            break;
        }

        // Run every tool call and gather the results into one user turn.
        let mut results = Vec::new();
        for block in &msg.content {
            if let ContentBlock::ToolUse(tu) = block {
                println!("  calling {} {}", tu.name.as_str(), tu.input);
                let output = weather_for(&tu.input);
                results.push(ContentBlockParam::ToolResult(ToolResultBlock::text(
                    tu.id.clone(),
                    output,
                )));
            }
        }
        history.push((Role::User, MessageContent::Blocks(results)));
    }

    Ok(())
}
