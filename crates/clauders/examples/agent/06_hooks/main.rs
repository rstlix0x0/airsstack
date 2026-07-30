//! Hooks: run Rust code at fixed points in the agent's loop.
//!
//! A hook is registered for one `HookEvent`, optionally narrowed by a matcher
//! string (a tool name, for the tool events). When that event fires the binary
//! issues a callback and waits for this program's `HookOutput`.
//!
//! `HookOutput` can observe silently (the default, empty output), inject a
//! `system_message` into the conversation, or veto the step with
//! `HookDecision::Block`.
//!
//! Hooks and permission policies are different levers: a policy answers "may
//! this tool run", a hook runs *around* the step and can also fire on events
//! that have nothing to do with tools (session start/end, prompt submission,
//! compaction).
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_06_hooks
//! ```

use std::sync::Arc;

use clauders::agent::cancel::CancelSignal;
use clauders::agent::error::AgentError;
use clauders::agent::{
    ContentBlock, Hook, HookDecision, HookEvent, HookInput, HookOutput, Message, Options,
    PermissionMode, query,
};
use futures_util::StreamExt;

/// Logs every event it is registered for and lets the step proceed.
struct Trace {
    /// Label printed with each line, so several registrations stay tellable
    /// apart in the output.
    label: &'static str,
}

#[async_trait::async_trait]
impl Hook for Trace {
    async fn call(
        &self,
        input: HookInput,
        _cancel: CancelSignal,
    ) -> Result<HookOutput, AgentError> {
        println!(
            "[hook {}] {:?} tool_use_id={:?}",
            self.label, input.event, input.tool_use_id
        );
        // Default output: no decision, no message — the step proceeds.
        Ok(HookOutput::default())
    }
}

/// Blocks any `Bash` command containing a word from a small denylist.
struct BlockDangerousBash;

#[async_trait::async_trait]
impl Hook for BlockDangerousBash {
    async fn call(
        &self,
        input: HookInput,
        _cancel: CancelSignal,
    ) -> Result<HookOutput, AgentError> {
        // The payload is the binary's own event body, opaque to the SDK.
        let command = input
            .data
            .get("tool_input")
            .and_then(|tool_input| tool_input.get("command"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();

        if ["rm ", "mv ", "dd ", "curl ", "chmod "]
            .iter()
            .any(|bad| command.contains(bad))
        {
            println!("[hook block] refusing `{command}`");
            return Ok(HookOutput {
                decision: Some(HookDecision::Block),
                reason: Some(format!("`{command}` is on this session's denylist")),
                system_message: Some(
                    "A hook blocked that command. Suggest a read-only alternative.".to_owned(),
                ),
                ..HookOutput::default()
            });
        }
        Ok(HookOutput::default())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::builder()
        .system_prompt("Use Bash when asked. Keep answers to one line.")
        .permission_mode(PermissionMode::BypassPermissions)
        .allowed_tools(vec!["Bash".to_owned()])
        // Narrowed to one tool: fires only for `Bash`.
        .hook(
            HookEvent::PreToolUse,
            Some("Bash".to_owned()),
            Arc::new(BlockDangerousBash),
        )
        // Unnarrowed: fires for every tool.
        .hook(
            HookEvent::PreToolUse,
            None,
            Arc::new(Trace { label: "pre" }),
        )
        .hook(
            HookEvent::PostToolUse,
            None,
            Arc::new(Trace { label: "post" }),
        )
        .hook(HookEvent::Stop, None, Arc::new(Trace { label: "stop" }))
        .hook(
            HookEvent::UserPromptSubmit,
            None,
            Arc::new(Trace { label: "prompt" }),
        )
        // Ask the binary to also emit hook-lifecycle frames on the message
        // stream. They arrive as `Message::Other` — this SDK does not model
        // their shape.
        .include_hook_events(true)
        .max_turns(6)
        .build();

    let mut stream = query(
        "Run `pwd`, then try to run `rm -rf ./scratch`, then stop.",
        options,
    )
    .await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => {
                for block in &assistant.content {
                    match block {
                        ContentBlock::Text { text } => println!("{text}"),
                        ContentBlock::ToolUse { name, input, .. } => {
                            println!("[requested] {name} {input}");
                        }
                        _ => {}
                    }
                }
            }
            Message::Other(raw) => {
                if let Some(kind) = raw.get("type").and_then(serde_json::Value::as_str) {
                    println!("[lifecycle frame] {kind}");
                }
            }
            Message::Result(result) => {
                println!("---");
                println!("{}", result.result);
            }
            _ => {}
        }
    }

    Ok(())
}
