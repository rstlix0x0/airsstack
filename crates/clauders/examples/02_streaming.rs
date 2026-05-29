//! SSE streaming Messages API request.
//!
//! Prints each text fragment to stdout as it arrives.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 02_streaming --features messages-streaming,transport-reqwest
//! ```

use std::pin::Pin;

use clauders::messages::{ContentDelta, StreamEvent};
use clauders::prelude::*;
use futures_core::Stream as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(1024)?;

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(max_tokens)
        .add_user_text("Count from one to five.")
        .build();

    let mut stream = client.messages().stream(req).await?;

    loop {
        let next = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        match next {
            None => break,
            Some(Ok(StreamEvent::ContentBlockDelta {
                delta: ContentDelta::TextDelta { text },
                ..
            })) => {
                use std::io::Write as _;
                print!("{text}");
                std::io::stdout().flush()?;
            }
            Some(Ok(_)) => {}
            Some(Err(e)) => return Err(e.into()),
        }
    }

    println!();
    Ok(())
}
