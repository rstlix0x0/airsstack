//! Deciding tool calls in Rust: a `PermissionPolicy` that allows, rewrites,
//! and denies.
//!
//! With a policy registered, the binary asks *this program* before it runs a
//! gated tool. The policy is consulted per call and returns one of:
//!
//! - `PermissionDecision::allow()` — run it unchanged.
//! - `PermissionDecision::allow_with(input)` — run it with rewritten arguments.
//! - `PermissionDecision::deny(msg)` — refuse this call, let the turn continue.
//! - `PermissionDecision::deny_interrupt(msg)` — refuse and abort the turn.
//!
//! A decision can also carry `PermissionUpdate`s that persist rules for the
//! rest of the session, which is how "always allow this" is expressed.
//!
//! Run (in a scratch directory — the agent is allowed to run `ls`):
//!
//! ```text
//! cargo run -p clauders --example agent_05_permissions
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use clauders::agent::cancel::CancelSignal;
use clauders::agent::error::AgentError;
use clauders::agent::{
    ContentBlock, Message, Options, PermissionBehavior, PermissionContext, PermissionDecision,
    PermissionMode, PermissionPolicy, PermissionRuleValue, PermissionUpdate,
    PermissionUpdateDestination, query,
};
use futures_util::StreamExt;

/// Allows read-only shell commands, rewrites `ls` to be long-form, and refuses
/// everything else.
struct ReadOnlyShell {
    /// How many calls the policy has judged.
    seen: AtomicUsize,
}

#[async_trait::async_trait]
impl PermissionPolicy for ReadOnlyShell {
    async fn can_use_tool(
        &self,
        tool: &str,
        input: &serde_json::Value,
        ctx: PermissionContext,
        cancel: CancelSignal,
    ) -> Result<PermissionDecision, AgentError> {
        let n = self.seen.fetch_add(1, Ordering::Relaxed) + 1;
        println!("[policy #{n}] {tool} {input}");
        if let Some(title) = &ctx.title {
            println!("[policy #{n}] the binary titled this: {title}");
        }
        for suggestion in &ctx.suggestions {
            println!("[policy #{n}] binary suggests: {suggestion:?}");
        }

        // Cancellation is cooperative: the binary may withdraw the request
        // while a slow policy is still thinking. Nothing kills the task, so a
        // policy that cares must check.
        if cancel.is_cancelled() {
            return Err(AgentError::Interrupted);
        }

        if tool != "Bash" {
            return Ok(PermissionDecision::deny(format!(
                "this example only permits Bash, not {tool}"
            )));
        }

        let command = input
            .get("command")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();

        // Rewrite: the model asked for `ls`, we run `ls -la` instead. The model
        // sees the output of what actually ran.
        if command.trim() == "ls" {
            let mut rewritten = input.clone();
            rewritten["command"] = serde_json::Value::String("ls -la".to_owned());
            println!("[policy #{n}] rewriting `ls` to `ls -la`");
            return Ok(PermissionDecision::allow_with(rewritten));
        }

        // Allow, and persist a session rule so the same command is not asked
        // about again.
        if command.starts_with("pwd") || command.starts_with("echo ") {
            return Ok(PermissionDecision::Allow {
                updated_input: None,
                updated_permissions: vec![PermissionUpdate::AddRules {
                    rules: vec![PermissionRuleValue {
                        tool_name: "Bash".to_owned(),
                        rule_content: Some(command.to_owned()),
                    }],
                    behavior: PermissionBehavior::Allow,
                    destination: PermissionUpdateDestination::Session,
                }],
            });
        }

        // Anything that could mutate the machine is refused outright. Using
        // `deny_interrupt` instead would end the whole turn here.
        Ok(PermissionDecision::deny(format!(
            "`{command}` is not a read-only command"
        )))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = Arc::new(ReadOnlyShell {
        seen: AtomicUsize::new(0),
    });

    let options = Options::builder()
        .system_prompt("Use the Bash tool to answer. Do not explain, just report what you saw.")
        // `Default` means "ask" — which is what routes calls to the policy.
        // `BypassPermissions` would skip it entirely.
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec!["Bash".to_owned()])
        .permission_policy(policy)
        .max_turns(6)
        .build();

    let mut stream = query(
        "Run `ls` to list this directory, then run `rm -rf /tmp/does-not-exist`.",
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
                        ContentBlock::ToolResult { is_error, .. } => {
                            println!("[tool result] is_error={is_error}");
                        }
                        _ => {}
                    }
                }
            }
            Message::Result(result) => {
                println!("---");
                println!("denials recorded: {}", result.permission_denials.len());
                for denial in &result.permission_denials {
                    println!("  {denial}");
                }
            }
            _ => {}
        }
    }

    Ok(())
}
