//! Two ways to shape *when* work starts: warm start, and streaming input.
//!
//! **Warm start** (`Client::startup`) pays the spawn-and-handshake cost before
//! the prompt is known. The returned `WarmQuery` accepts exactly one prompt —
//! enforced by the type, since `query` consumes it — and hands back a live
//! session.
//!
//! **Streaming input** (`Prompt::stream`) feeds user turns into a session as
//! they arrive instead of sending one fixed prompt. Useful when turns come from
//! a UI, a socket, or another task.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_11_streaming_input
//! ```

use std::time::Duration;

use clauders::agent::types::Prompt;
use clauders::agent::{Client, ContentBlock, Message, Options, PermissionMode};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    warm_start().await?;
    println!("=== streaming input ===");
    streaming_input().await?;
    Ok(())
}

/// Spawn and handshake up front, then send the single prompt when it is ready.
async fn warm_start() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== warm start ===");

    let options = Options::builder()
        .system_prompt("Answer in one line.")
        .permission_mode(PermissionMode::Plan)
        .max_turns(2)
        .build();

    let started = std::time::Instant::now();
    let warm = Client::startup(options).await?;
    println!(
        "session ready in {:?} (no prompt sent yet)",
        started.elapsed()
    );

    // Something else could happen here — reading input, waiting on a queue —
    // while the subprocess is already up.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // `query` consumes the handle: a second call does not compile. Use
    // `WarmQuery::close()` to tear the session down without ever querying.
    let mut session = warm.query("Name one Rust crate for async I/O.").await?;

    while let Some(frame) = session.stream().next().await {
        match frame? {
            Message::Assistant(assistant) => print_text(&assistant.content),
            Message::Result(_) => break,
            _ => {}
        }
    }

    // The warm session also exposes `interrupt()` for the turn it is running.
    println!("first token to result in {:?}", started.elapsed());
    Ok(())
}

/// Feed several user turns into one live session as they become available.
async fn streaming_input() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::builder()
        .system_prompt("Answer each message in one short line. Keep track of the running total.")
        .permission_mode(PermissionMode::Plan)
        .max_turns(8)
        .build();

    let client = Client::connect(options).await?;

    // Any `Stream<Item = String>` works. Here the turns are spaced out to
    // stand in for input that genuinely arrives over time.
    let turns = futures_util::stream::iter([
        "Remember the number 7.".to_owned(),
        "Add 5 to it.".to_owned(),
        "What is the running total?".to_owned(),
    ])
    .then(|text| async move {
        tokio::time::sleep(Duration::from_millis(400)).await;
        println!("> {text}");
        text
    });

    let mut stream = client.query(Prompt::stream(turns)).await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => print_text(&assistant.content),
            Message::Result(result) => {
                println!("---");
                println!("{} turn(s): {}", result.num_turns, result.result);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Print the text blocks of one assistant turn.
fn print_text(blocks: &[ContentBlock]) {
    for block in blocks {
        if let ContentBlock::Text { text } = block {
            println!("{text}");
        }
    }
}
