//! Token-level streaming, child stderr, and forward compatibility.
//!
//! By default the stream carries whole messages: one `Assistant` frame per
//! model turn. `include_partial_messages(true)` additionally emits the
//! underlying `StreamEvent` frames, so text can be printed as it is produced.
//!
//! Three things are shown together because they are all about *observing* a
//! session rather than configuring it:
//!
//! - `Message::StreamEvent` — the raw partial-message events.
//! - `Options::stderr` — a callback fed each chunk the child writes to stderr.
//! - `Message::Other` — any frame this SDK release does not model, kept
//!   verbatim instead of failing the turn.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_04_partial_streaming
//! ```

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use clauders::agent::{ContentBlock, Message, Options, PermissionMode, query};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The stderr callback runs on the runtime's reader task, so anything it
    // touches must be `Send + Sync`. Counting bytes keeps the output readable;
    // printing the chunk itself works just as well.
    let stderr_bytes = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&stderr_bytes);

    let options = Options::builder()
        .system_prompt("Answer in two or three sentences.")
        .permission_mode(PermissionMode::Plan)
        .include_partial_messages(true)
        .stderr(move |chunk: &str| {
            counter.fetch_add(chunk.len(), Ordering::Relaxed);
        })
        .build();

    let mut stream = query("Explain what a Rust lifetime is.", options).await?;

    // Deltas for the turn currently streaming, and the total across the run.
    let mut turn_deltas = 0_usize;
    let mut total_deltas = 0_usize;

    while let Some(frame) = stream.next().await {
        match frame? {
            // Partial frames: one per token-ish chunk. The payload is opaque
            // JSON in the shape the Messages API streams, so a caller reads
            // whichever fields it cares about. Printing here — not on the
            // completed frame below — is what makes the output appear as the
            // model produces it.
            Message::StreamEvent(event) => {
                if let Some(text) = delta_text(&event.event) {
                    turn_deltas += 1;
                    total_deltas += 1;
                    print!("{text}");
                    std::io::stdout().flush()?;
                }
            }
            // The whole message arrives once the turn completes, and it is the
            // authoritative copy either way. When deltas did stream it is a
            // duplicate, so only its size is reported; when none arrived it is
            // the only copy, so it is printed in full. A program that does not
            // care about latency can ignore partials and use just this.
            Message::Assistant(assistant) => {
                if turn_deltas == 0 {
                    for block in &assistant.content {
                        if let ContentBlock::Text { text } = block {
                            println!("{text}");
                        }
                    }
                    println!("[no partial frames for this turn; printed the complete frame]");
                } else {
                    let bytes: usize = assistant
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(text.len()),
                            _ => None,
                        })
                        .sum();
                    println!();
                    println!(
                        "[streamed {turn_deltas} delta(s); complete frame carries {bytes} bytes]"
                    );
                }
                turn_deltas = 0;
            }
            Message::Result(result) => {
                println!("---");
                println!("text deltas seen: {total_deltas}");
                println!("stderr bytes:     {}", stderr_bytes.load(Ordering::Relaxed));
                if let Some(ttft) = result.ttft_ms {
                    println!("time to first token: {ttft} ms");
                }
                if total_deltas == 0 {
                    println!(
                        "note: no partial frames arrived. `include_partial_messages` asks the \
                         binary for them; this binary did not send any for this run."
                    );
                }
            }
            // Frames with a `type` this release does not model land here. Hook
            // lifecycle frames (see example 06) are the common case.
            Message::Other(raw) => {
                let kind = raw
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("<untyped>");
                println!("[unmodelled frame: {kind}]");
            }
            Message::User(_) | Message::System(_) => {}
        }
    }

    Ok(())
}

/// Pull the incremental text out of one partial-message event.
///
/// The payload is the Messages API's own streaming event, passed through by the
/// binary. A `content_block_delta` carries either `delta.text` (a `text_delta`)
/// or `delta.thinking` (a `thinking_delta`); every other event kind — block
/// start and stop, message start and delta — carries no text and yields `None`.
fn delta_text(event: &serde_json::Value) -> Option<&str> {
    let delta = event.get("delta")?;
    delta
        .get("text")
        .or_else(|| delta.get("thinking"))
        .and_then(serde_json::Value::as_str)
}
