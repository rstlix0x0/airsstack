//! Streaming chat completion over Server-Sent Events.
//!
//! Run:
//!
//! ```text
//! OPENROUTER_API_KEY=sk-... cargo run --example 02_streaming
//! ```

use std::io::Write;

use futures_util::StreamExt;
use openrouter_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    let req = ChatRequest::builder()
        .model(ModelId::custom("openai/gpt-4o-mini")?)
        .messages(vec![Message::user("Count from one to five.")])
        .build();

    let mut stream = client.chat().stream(req).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(choice) = chunk.choices.first() {
            if let Some(text) = &choice.delta.content {
                print!("{text}");
                std::io::stdout().flush()?;
            }
        }
    }
    println!();

    Ok(())
}
