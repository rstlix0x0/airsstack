//! Prompt caching across two calls — writes a cache block on the first
//! request, then reads it back on the second.
//!
//! The server only stores a cache block once the cached prefix exceeds a
//! minimum length (about 1024 tokens for Sonnet). The crate README alone is
//! below that, so it is repeated to clear the threshold. A real application
//! would cache a genuinely large, stable system prompt or document corpus.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 04_caching --features messages-caching,transport-reqwest
//! ```

use clauders::prelude::*;
use clauders::types::{CacheControl, SystemPrompt, SystemSegment};

// Stand-in for a long, stable system prompt.
const BASE_SYSTEM: &str = include_str!("../README.md");
// Repeat count chosen so the cached prefix clears the server's ~1024-token
// minimum; the README is a few hundred tokens, so four copies is ample.
const SYSTEM_REPEATS: usize = 4;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(64)?;
    let system_text = BASE_SYSTEM.repeat(SYSTEM_REPEATS);

    // The system prefix is identical on both requests, so the second call
    // reads the cache block the first call wrote.
    for label in [
        "first call (expect cache write)",
        "second call (expect cache read)",
    ] {
        let segment =
            SystemSegment::text(system_text.clone()).with_cache(CacheControl::ephemeral());
        let req = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(max_tokens)
            .system(SystemPrompt::segments(vec![segment]))
            .add_user_text("Reply with the single word: ok.")
            .build();

        let msg = client.messages().create(req).await?;

        println!("--- {label} ---");
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
    }

    Ok(())
}
