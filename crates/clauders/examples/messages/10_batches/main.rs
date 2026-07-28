//! Message Batches — submit many requests, poll, then stream the results.
//!
//! Creates a batch of two independent requests, polls the batch until it ends,
//! then streams the per-row results and prints each by its `custom_id`.
//! Batches run asynchronously server-side; a small batch usually finishes in
//! well under a minute, but it can take longer.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 10_batches
//! ```

use std::pin::Pin;
use std::time::Duration;

use clauders::messages::{BatchRequest, BatchResult, BatchStatus};
use clauders::prelude::*;
use clauders::types::CustomRequestId;
use futures_core::Stream as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(64);

    let one = |prompt: &str| {
        MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(max_tokens)
            .add_user_text(prompt)
            .build()
    };

    let batch_req = BatchRequest::builder()
        .add(
            CustomRequestId::new("greeting")?,
            one("Say hello in French."),
        )
        .add(
            CustomRequestId::new("math")?,
            one("What is 6 times 7? Reply with the number only."),
        )
        .build();

    let batch = client.messages().batches().create(batch_req).await?;
    println!("submitted batch {}", batch.id.as_str());

    // Poll until the batch reaches a terminal state.
    let ended = loop {
        let b = client.messages().batches().get(&batch.id).await?;
        let c = b.request_counts;
        println!(
            "status {:?} — processing {} succeeded {} errored {}",
            b.processing_status, c.processing, c.succeeded, c.errored
        );
        if b.processing_status == BatchStatus::Ended {
            break b;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    };

    // Stream the JSONL results, one row per submitted request.
    let mut stream = client.messages().batches().results(&ended.id).await?;
    println!("--- results ---");
    loop {
        let next = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        match next {
            None => break,
            Some(Ok(row)) => {
                print!("{}: ", row.custom_id.as_str());
                match row.result {
                    BatchResult::Succeeded { message } => {
                        for block in &message.content {
                            if let ContentBlock::Text(t) = block {
                                println!("{}", t.text);
                            }
                        }
                    }
                    BatchResult::Errored { error } => println!("errored: {}", error.message),
                    BatchResult::Canceled => println!("canceled"),
                    BatchResult::Expired => println!("expired"),
                }
            }
            Some(Err(e)) => return Err(e.into()),
        }
    }

    Ok(())
}
