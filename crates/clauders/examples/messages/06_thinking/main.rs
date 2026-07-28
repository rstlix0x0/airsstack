//! Extended thinking — let the model reason before answering.
//!
//! Turns on thinking with an explicit token budget, then reads the thinking
//! block and the final answer separately from the response content.
//!
//! The budget must be at least 1024 tokens and strictly less than the
//! request's `max_tokens`; the server rejects a budget that violates either
//! bound. Not every model supports extended thinking.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 06_thinking
//! ```

use clauders::messages::ContentBlock;
use clauders::messages::ThinkingConfig;
use clauders::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    // Budget (1024) stays below max_tokens (4096), as the API requires.
    let max_tokens = MaxTokens::new(4096);
    let thinking = ThinkingConfig::enabled(1024);

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(max_tokens)
        .thinking(thinking)
        .add_user_text(
            "A farmer has 17 sheep. All but 9 run away. How many are left? Think it through.",
        )
        .build();

    let msg = client.messages().create(req).await?;

    // Thinking and the answer are separate blocks in the response content.
    for block in &msg.content {
        match block {
            ContentBlock::Thinking(t) => {
                println!("--- thinking ---");
                println!("{}", t.thinking);
            }
            ContentBlock::Text(t) => {
                println!("--- answer ---");
                println!("{}", t.text);
            }
            _ => {}
        }
    }

    println!(
        "usage: input={} output={}",
        msg.usage.input_tokens, msg.usage.output_tokens
    );
    Ok(())
}
