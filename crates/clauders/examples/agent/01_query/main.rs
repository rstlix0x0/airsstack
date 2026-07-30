//! The smallest Agent SDK program: one prompt, one session, streamed frames.
//!
//! Unlike the Messages API examples there is no API key here. The Agent SDK
//! spawns the `claude` binary as a subprocess and talks to it over the control
//! protocol, so it reuses whatever credentials that binary already has.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_01_query
//! ```

use clauders::agent::{ContentBlock, Message, Options, query};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = query("Say hi in one short sentence.", Options::default()).await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => {
                for block in &assistant.content {
                    if let ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
            Message::Result(result) => {
                println!("---");
                println!("session:  {}", result.session_id.as_str());
                println!("turns:    {}", result.num_turns);
                println!("is_error: {}", result.is_error);
                if let Some(cost) = result.total_cost_usd {
                    println!("cost:     ${cost:.6}");
                }
                if let Some(usage) = &result.usage {
                    println!(
                        "usage:    input={} output={}",
                        usage.input_tokens, usage.output_tokens
                    );
                }
            }
            Message::User(_) | Message::System(_) | Message::StreamEvent(_) => {}
            Message::Other(raw) => println!("(unmodelled frame: {raw})"),
        }
    }

    Ok(())
}
