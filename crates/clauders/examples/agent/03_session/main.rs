//! A stateful session: connect once, take several turns, then read what the
//! handshake told us and what the session cost.
//!
//! `query` from example 01 is sugar for connect-send-drop. `Client::connect`
//! keeps the subprocess alive, so later turns see the earlier ones and the
//! control operations stay available between turns.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_03_session
//! ```

use clauders::agent::{Client, ContentBlock, Message, Options, PermissionMode};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::builder()
        .system_prompt("Answer in one short sentence. Never use a tool.")
        .permission_mode(PermissionMode::Plan)
        .max_turns(2)
        .build();

    // Two equivalent entry points; the builder form is handy when options are
    // assembled in one place and the connection happens in another.
    let client = Client::builder().options(options).connect().await?;

    // Everything below is answered from the retained `initialize` response —
    // no round-trip to the subprocess.
    println!("advertised models:   {}", client.supported_models().len());
    println!("slash commands:      {}", client.supported_commands().len());
    println!("on-disk agents:      {}", client.supported_agents().len());
    println!(
        "supports hooks flag: {}",
        client.capabilities().supports("hooks")
    );
    for command in client.supported_commands().iter().take(5) {
        if let Some(name) = command.get("name").and_then(serde_json::Value::as_str) {
            println!("  /{name}");
        }
    }
    println!("---");

    // Two turns on one session. The second relies on the first for context.
    for prompt in [
        "Pick a number between 1 and 10 and tell me what it is.",
        "Double the number you just picked.",
    ] {
        println!("> {prompt}");
        let mut stream = client.query(prompt).await?;
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
                    println!("(session {})", result.session_id.as_str());
                }
                _ => {}
            }
        }
        println!("---");
    }

    // Live control requests over the same session.
    let usage = client.get_usage().await?;
    println!("subscription:   {:?}", usage.subscription_type);
    println!("rate limits on: {}", usage.rate_limits_available);
    println!("session totals: {}", usage.session);

    let context = client.get_context_usage().await?;
    println!(
        "context:        {} / {} tokens ({}%) on {}",
        context.total_tokens, context.max_tokens, context.percentage, context.model
    );

    // Dropping the client tears the subprocess down within `shutdown_grace`.
    drop(client);
    Ok(())
}
