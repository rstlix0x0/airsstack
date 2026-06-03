//! Tool-call round-trip: the model requests a function call, the program
//! answers, and the model produces a final reply.
//!
//! Run:
//!
//! ```text
//! OPENROUTER_API_KEY=sk-... cargo run --example 03_tools
//! ```

use openrouter_rs::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let model = ModelId::custom("openai/gpt-4o-mini")?;

    // Define a single weather-lookup tool.
    let weather = Tool::function(FunctionDef {
        name: FunctionName::new("get_weather")?,
        description: Some("Get the current temperature for a city.".into()),
        parameters: Some(json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"]
        })),
        strict: None,
    });

    // First turn: ask a question that should trigger the tool.
    let first = ChatRequest::builder()
        .model(model.clone())
        .messages(vec![Message::user("What is the weather in Paris?")])
        .tools(vec![weather.clone()])
        .tool_choice(ToolChoice::Auto)
        .build();

    let completion = client.chat().send(first).await?;
    let choice = completion.choices.first().ok_or("no choice returned")?;
    let calls = choice
        .message
        .tool_calls
        .as_ref()
        .ok_or("model did not request a tool call")?;
    let call = calls.first().ok_or("empty tool_calls")?;
    println!(
        "model called: {}({})",
        call.function.name, call.function.arguments
    );

    // Second turn: replay the assistant's tool-call message, then answer it.
    let call_id = ToolCallId::new(call.id.as_str())?;
    let second = ChatRequest::builder()
        .model(model)
        .messages(vec![
            Message::user("What is the weather in Paris?"),
            Message::assistant_tool_calls(calls.clone()),
            Message::tool_result(call_id, "18 degrees Celsius, clear skies."),
        ])
        .tools(vec![weather])
        .build();

    let final_completion = client.chat().send(second).await?;
    if let Some(text) = final_completion
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
    {
        println!("final: {text}");
    }

    Ok(())
}
