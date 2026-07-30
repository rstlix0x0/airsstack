//! Subagents defined in Rust: named helpers the main agent can delegate to.
//!
//! An `AgentDefinition` is registered under the name the model invokes. Its
//! `description` is what the model reads when choosing a helper, and its
//! `prompt` is that helper's own system prompt. Every other field overrides the
//! parent session when set and inherits it when not — which makes a subagent
//! the natural place to run a narrow task on a cheaper model with fewer tools.
//!
//! `agent_name` is the separate lever: it selects *one* agent to run as the
//! session itself, instead of the default assistant.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_10_subagents
//! ```

use clauders::agent::{
    AgentDefinition, ContentBlock, MemorySource, Message, Options, PermissionMode, query,
};
use clauders::types::{EffortLevel, ModelId};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Required fields are validated at construction: an empty description or
    // prompt is an error here, not a confusing failure later.
    let summarizer = AgentDefinition::new(
        "Summarizes a file in three bullet points. Use for any 'summarize this file' request.",
        "You summarize source files. Read the file, then reply with exactly three bullet points.",
    )?
    // Narrow the tools: this helper reads, it does not write or run commands.
    .with_tools(vec!["Read".to_owned(), "Glob".to_owned()])
    // Run the cheap model for a cheap task; the parent session keeps its own.
    .with_model(ModelId::claude_haiku_4_5())
    .with_effort(EffortLevel::Low)
    .with_max_turns(3)
    .with_permission_mode(PermissionMode::Plan);

    let counter = AgentDefinition::new(
        "Counts things in a directory — files, lines, matches.",
        "You answer counting questions with a single number and nothing else.",
    )?
    .with_tools(vec!["Bash".to_owned(), "Glob".to_owned()])
    // Remove a tool from whatever the effective set turns out to be.
    .with_disallowed_tools(vec!["WebFetch".to_owned()])
    // Which memory scope this helper reads (`user`, `project`, or `local`).
    .with_memory(MemorySource::Project)
    .with_max_turns(2);

    println!("registered helpers:");
    for (name, definition) in [("summarizer", &summarizer), ("counter", &counter)] {
        println!(
            "  {name}: model={:?} tools={:?} max_turns={:?}",
            definition.model().map(ModelId::as_str),
            definition.tools(),
            definition.max_turns()
        );
    }
    println!("---");

    let options = Options::builder()
        .system_prompt(
            "You coordinate helpers. Delegate to the `counter` helper rather than counting \
             yourself, then report its answer.",
        )
        .permission_mode(PermissionMode::BypassPermissions)
        .agent("summarizer", summarizer)
        .agent("counter", counter)
        .allowed_tools(vec![
            "Task".to_owned(),
            "Bash".to_owned(),
            "Glob".to_owned(),
        ])
        .max_turns(10)
        .build();

    let mut stream = query(
        "How many .rs files are under this crate's src directory?",
        options,
    )
    .await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => {
                // A turn produced *by* a subagent carries the id of the tool
                // call that spawned it, so delegated work is attributable.
                let origin = assistant
                    .parent_tool_use_id
                    .as_deref()
                    .map_or_else(|| "main".to_owned(), |id| format!("subagent {id}"));
                for block in &assistant.content {
                    match block {
                        ContentBlock::Text { text } => println!("[{origin}] {text}"),
                        ContentBlock::ToolUse { name, input, .. } => {
                            println!("[{origin}] calls {name} {input}");
                        }
                        _ => {}
                    }
                }
            }
            Message::Result(result) => {
                println!("---");
                println!("{}", result.result);
                println!("turns: {}", result.num_turns);
            }
            _ => {}
        }
    }

    Ok(())
}
