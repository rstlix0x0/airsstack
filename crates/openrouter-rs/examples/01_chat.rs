//! Minimal non-streaming chat completion.
//!
//! Run:
//!
//! ```text
//! OPENROUTER_API_KEY=sk-... cargo run --example 01_chat
//! ```

use openrouter_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    let req = ChatRequest::builder()
        .model(ModelId::custom("openai/gpt-4o-mini")?)
        .messages(vec![Message::user("Say hi in one word.")])
        .build();

    let completion = client.chat().send(req).await?;

    if let Some(choice) = completion.choices.first() {
        if let Some(text) = &choice.message.content {
            println!("{text}");
        }
        println!("finish_reason: {:?}", choice.finish_reason);
    }
    if let Some(usage) = &completion.usage {
        println!(
            "usage: prompt={} completion={} total={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}
