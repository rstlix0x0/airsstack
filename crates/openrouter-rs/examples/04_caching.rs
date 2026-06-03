//! Edge-cache demo: the same request is sent twice through `send_cached`.
//! The first call is a cache MISS (write); the second should report a HIT.
//!
//! A real cache write requires a prompt large enough to exceed the gateway's
//! minimum cacheable size, so this example sends a long system prompt.
//!
//! Run:
//!
//! ```text
//! OPENROUTER_API_KEY=sk-... cargo run --example 04_caching
//! ```

use openrouter_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let model = ModelId::custom("openai/gpt-4o-mini")?;

    // A long, stable system prompt so the request is large enough to cache.
    let system_prompt = "You are a helpful assistant. ".repeat(64);

    let build_request = || {
        ChatRequest::builder()
            .model(model.clone())
            .messages(vec![
                Message::system(system_prompt.clone()),
                Message::user("Reply with the single word: ready."),
            ])
            .build()
    };

    for label in ["first (expect MISS)", "second (expect HIT)"] {
        let cached = client
            .chat()
            .send_cached(build_request(), ResponseCache::enabled())
            .await?;
        println!(
            "{label}: status={:?} age_secs={:?} ttl_secs={:?}",
            cached.status, cached.age_secs, cached.ttl_secs
        );
        if let Some(usage) = &cached.value.usage {
            println!("  usage: total_tokens={}", usage.total_tokens);
        }
    }

    Ok(())
}
