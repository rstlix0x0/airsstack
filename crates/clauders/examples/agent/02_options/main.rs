//! A tour of the basic session `Options`: prompt, model, tool gating, working
//! directory, environment, turn cap, and the two subprocess timeouts.
//!
//! Every field shown here is set before the session starts. `Options` is the
//! single argument both `query` and `Client::connect` take, so the same value
//! configures a one-shot run and a long-lived session.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_02_options
//! ```

use std::time::Duration;

use clauders::agent::{ContentBlock, Message, Options, PermissionMode, query};
use clauders::types::{MaxTokens, ModelId};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;

    let options = Options::builder()
        // What the agent is told about itself, before any user turn.
        .system_prompt("You are a terse code reader. Answer in at most three lines.")
        // Model and per-request output ceiling.
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(2048))
        // Tool gating. `Plan` proposes without executing; the allowlist and
        // denylist narrow what the binary will even offer the model.
        .permission_mode(PermissionMode::Plan)
        .allowed_tools(vec![
            "Read".to_owned(),
            "Grep".to_owned(),
            "Glob".to_owned(),
        ])
        .disallowed_tools(vec!["Bash".to_owned()])
        // Where the subprocess runs, and what else it may read.
        .cwd(&cwd)
        .add_dir(cwd.join("src"))
        // Extra environment for the child process only.
        .env("CLAUDERS_EXAMPLE", "agent_02_options")
        // Stop after this many agent turns even if the task is unfinished.
        .max_turns(4)
        // Two independent subprocess timeouts.
        .control_request_timeout(Duration::from_secs(30))
        .shutdown_grace(Duration::from_secs(2))
        // Refuse to run against a `claude` binary older than the supported
        // minimum instead of warning and continuing.
        .require_min_version(true)
        .build();

    // `Options` has a hand-written `Debug` that shows configuration but never
    // the contents of a registered handler.
    println!("{options:#?}");
    println!("---");

    let mut stream = query(
        "Which files are in this crate's src/agent directory?",
        options,
    )
    .await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => print_blocks(&assistant.content),
            Message::Result(result) => {
                println!("---");
                println!("turns:   {} (capped at 4)", result.num_turns);
                println!("subtype: {:?}", result.subtype);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Print the text the model produced and name any tool it asked to run.
fn print_blocks(blocks: &[ContentBlock]) {
    for block in blocks {
        match block {
            ContentBlock::Text { text } => println!("{text}"),
            ContentBlock::ToolUse { name, input, .. } => println!("[tool] {name} {input}"),
            _ => {}
        }
    }
}
