//! Minimal non-streaming Messages API request.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 01_hello --features messages,transport-reqwest
//! ```

use clauders::messages::ContentBlock;
use clauders::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(1024)?;

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(max_tokens)
        .add_user_text("Say hi.")
        .build();

    let msg = client.messages().create(req).await?;

    for block in &msg.content {
        if let ContentBlock::Text(t) = block {
            println!("{}", t.text);
        }
    }

    println!("stop_reason: {:?}", msg.stop_reason);
    println!(
        "usage: input={} output={}",
        msg.usage.input_tokens, msg.usage.output_tokens
    );

    Ok(())
}
