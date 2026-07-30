//! Driving a session while it runs: interrupt, switch model, change permission
//! mode, inspect MCP, read files, and re-read the handshake.
//!
//! Every method here is a control request over the same pipe the messages come
//! down. They are available whenever the `Client` is, including mid-turn — which
//! is the whole point of `interrupt`.
//!
//! Some of them are refused in a plain session (there is no task to stop, no
//! MCP server to reconnect, no user message to rewind to). Those calls print
//! the refusal instead of aborting, because the refusal is the interesting part:
//! it is the binary answering, not the SDK guessing.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_12_live_control
//! ```

use std::fmt::Debug;
use std::time::Duration;

use clauders::agent::{Client, ContentBlock, Message, Options, PermissionMode};
use clauders::types::ModelId;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::builder()
        .system_prompt("Work slowly and narrate each step.")
        .permission_mode(PermissionMode::Plan)
        .max_turns(12)
        .build();

    let client = Client::connect(options).await?;

    interrupt_a_running_turn(&client).await?;
    println!("=== session-level control ===");
    session_control(&client).await?;
    println!("=== inspection ===");
    inspection(&client).await?;
    println!("=== workspace and tasks ===");
    workspace_and_tasks(&client).await?;

    Ok(())
}

/// Start a long turn, then cut it short from the same task.
async fn interrupt_a_running_turn(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== interrupt ===");

    let mut stream = client
        .query("Count slowly from 1 to 200, one number per line, with a short comment on each.")
        .await?;

    let mut cutoff = Box::pin(tokio::time::sleep(Duration::from_secs(4)));
    let mut sent = false;

    loop {
        tokio::select! {
            // Fires once, four seconds in, whatever the turn is doing.
            () = &mut cutoff, if !sent => {
                sent = true;
                match client.interrupt().await? {
                    // Newer binaries report what was still queued.
                    Some(receipt) => println!(
                        "[interrupt] sent; {} item(s) still queued",
                        receipt.still_queued.len()
                    ),
                    None => println!("[interrupt] sent; this binary reports no queue detail"),
                }
            }
            frame = stream.next() => {
                match frame {
                    None => break,
                    Some(frame) => match frame? {
                        Message::Assistant(assistant) => {
                            for block in &assistant.content {
                                if let ContentBlock::Text { text } = block {
                                    // Only the first line, to keep the log short.
                                    let head = text.lines().next().unwrap_or_default();
                                    println!("[turn] {head}");
                                }
                            }
                        }
                        Message::Result(result) => {
                            println!("[result] subtype={:?} turns={}", result.subtype, result.num_turns);
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}

/// Change what the session is and how it gates tools, without reconnecting.
async fn session_control(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    report(
        "set_model(haiku)",
        client.set_model(ModelId::claude_haiku_4_5()).await,
    );
    report(
        "set_permission_mode(acceptEdits)",
        client
            .set_permission_mode(PermissionMode::AcceptEdits)
            .await,
    );
    report(
        "set_max_thinking_tokens(1024)",
        client.set_max_thinking_tokens(Some(1024), None).await,
    );
    report(
        "apply_flag_settings",
        client
            .apply_flag_settings(serde_json::json!({ "verbose": true }))
            .await,
    );
    Ok(())
}

/// Read back what the session currently is.
async fn inspection(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let status = client.mcp_status().await?;
    println!("mcp servers: {}", status.servers.len());
    for server in &status.servers {
        println!("  {} -> {:?}", server.name, server.status);
    }

    // Refused without a configured server of that name — which is the point.
    report(
        "reconnect_mcp_server(\"nope\")",
        client.reconnect_mcp_server("nope").await,
    );
    report(
        "toggle_mcp_server(\"nope\", false)",
        client.toggle_mcp_server("nope", false).await,
    );

    let context = client.get_context_usage().await?;
    println!(
        "context: {} / {} tokens ({}%), auto-compact={}",
        context.total_tokens,
        context.max_tokens,
        context.percentage,
        context.is_auto_compact_enabled
    );

    let usage = client.get_usage().await?;
    println!(
        "usage: rate_limits_available={}",
        usage.rate_limits_available
    );

    report("reload_skills", client.reload_skills().await);
    report("reload_plugins", client.reload_plugins().await);

    // Re-runs the handshake over the live channel; the cheap accessors
    // (`supported_models`, `capabilities`, …) read the retained copy instead.
    let initialized = client.reinitialize().await?;
    println!(
        "reinitialize: {} model(s), {} command(s)",
        initialized.models.len(),
        initialized.commands.len()
    );

    Ok(())
}

/// File and task operations that go through the binary rather than the SDK.
async fn workspace_and_tasks(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    // Reading through the binary — rather than with `std::fs` — is what makes a
    // later edit of the same path acceptable to the agent's own read-tracking.
    let manifest = std::env::current_dir()?.join("Cargo.toml");
    match client
        .read_file(&manifest.to_string_lossy(), Some(256), None)
        .await
    {
        Ok(file) => println!(
            "read_file: {} ({} bytes read, truncated={:?})",
            file.abs_path,
            file.contents.len(),
            file.truncated
        ),
        Err(error) => println!("read_file: refused -> {error}"),
    }

    // Move any foreground tool call to the background. `None` targets them all.
    report("background_tasks(all)", client.background_tasks(None).await);

    // No such task, so this reports a refusal.
    report("stop_task(\"task_0\")", client.stop_task("task_0").await);

    // Rewind wants the uuid of a *user* message in this session. A dry run
    // reports what would change; with a uuid the session does not know, the
    // binary answers `can_rewind: false` and says why.
    match client
        .rewind_files("00000000-0000-0000-0000-000000000000", Some(true))
        .await
    {
        Ok(rewind) => println!(
            "rewind_files (dry run): can_rewind={} error={:?}",
            rewind.can_rewind, rewind.error
        ),
        Err(error) => println!("rewind_files: refused -> {error}"),
    }

    Ok(())
}

/// Print an outcome without letting a refusal end the example.
fn report<T: Debug, E: std::fmt::Display>(label: &str, outcome: Result<T, E>) {
    match outcome {
        Ok(value) => println!("{label}: ok {value:?}"),
        Err(error) => println!("{label}: refused -> {error}"),
    }
}
