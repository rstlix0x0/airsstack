//! Cached system prompt using prompt caching.
//!
//! Uses the crate README as a stand-in for a long, stable system prompt.
//! On the second call the prompt should be served from the cache.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 04_caching --features messages-caching,transport-reqwest
//! ```

use clauders::prelude::*;
use clauders::types::{CacheControl, SystemPrompt, SystemSegment};

// Stand-in for a long, stable system prompt that benefits from caching.
const LONG_SYSTEM: &str = include_str!("../README.md");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(256)?;

    let seg = SystemSegment::text(LONG_SYSTEM).with_cache(CacheControl::ephemeral());
    let system = SystemPrompt::segments(vec![seg]);

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(max_tokens)
        .system(system)
        .add_user_text("Summarize what you know about this crate in one sentence.")
        .build();

    let msg = client.messages().create(req).await?;

    for block in &msg.content {
        if let clauders::messages::ContentBlock::Text(t) = block {
            println!("{}", t.text);
        }
    }

    println!("input_tokens:        {}", msg.usage.input_tokens);
    println!(
        "cache_creation:      {:?}",
        msg.usage.cache_creation_input_tokens
    );
    println!(
        "cache_read:          {:?}",
        msg.usage.cache_read_input_tokens
    );
    println!("total_input_tokens:  {}", msg.usage.total_input_tokens());

    Ok(())
}
