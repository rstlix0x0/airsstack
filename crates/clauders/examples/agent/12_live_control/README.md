# 12 — Live control

Driving a session while it runs: interrupt it, change the model, inspect MCP, read
files, poke at tasks, re-read the handshake.

## Run

```text
cargo run -p clauders --example agent_12_live_control
```

## What it shows

Every method here is a control request over the same pipe the messages come down. They
are available whenever the `Client` is — including mid-turn, which is the whole point
of `interrupt`.

Some of them are **refused** in a plain session: there is no task to stop, no MCP
server to reconnect, no user message to rewind to. The example prints those refusals
instead of aborting, because the refusal is the interesting part — it is the binary
answering, not the SDK guessing:

```rust
fn report<T: Debug, E: Display>(label: &str, outcome: Result<T, E>) {
    match outcome {
        Ok(value) => println!("{label}: ok {value:?}"),
        Err(error) => println!("{label}: refused -> {error}"),
    }
}
```

## Interrupting a running turn

```rust
let mut stream = client.query("Count slowly from 1 to 200…").await?;
let mut cutoff = Box::pin(tokio::time::sleep(Duration::from_secs(4)));
let mut sent = false;

loop {
    tokio::select! {
        () = &mut cutoff, if !sent => {
            sent = true;
            match client.interrupt().await? {
                Some(receipt) => println!("{} item(s) still queued", receipt.still_queued.len()),
                None => println!("this binary reports no queue detail"),
            }
        }
        frame = stream.next() => { /* … */ }
    }
}
```

The stream does not borrow the client, so consuming frames and issuing control requests
can happen in the same task under `select!`.

`interrupt()` returns `Option<InterruptReceipt>`. `Some` carries the ids still queued
after the interrupt; `None` means this binary does not report that detail. Absence is
a capability difference, not an error.

## Changing the session

```rust
client.set_model(ModelId::claude_haiku_4_5()).await?;
client.set_permission_mode(PermissionMode::AcceptEdits).await?;
client.set_max_thinking_tokens(Some(1024), None).await?;
client.apply_flag_settings(serde_json::json!({ "verbose": true })).await?;
```

These are the mid-session equivalents of `Options::model`, `permission_mode`, and the
thinking budget. Most startup options have no such equivalent — they are argv flags,
fixed at spawn.

## Inspection

```rust
let status = client.mcp_status().await?;
for server in &status.servers {
    println!("{} -> {:?}", server.name, server.status);
}
```

`ServerConnection` is `Connected`, `Failed`, `NeedsAuth`, `Pending`, `Disabled`, or
`Unknown(String)` for a state this release does not model.

The MCP set can also be changed live: `reconnect_mcp_server(name)`,
`toggle_mcp_server(name, enabled)`, `set_mcp_servers(json)` — which returns what was
added, removed, and any per-server error — and
`set_mcp_permission_mode_override(name, mode)`.

```rust
client.get_context_usage().await?;   // context-window breakdown
client.get_usage().await?;           // cost / rate-limit utilization
client.reload_skills().await?;
client.reload_plugins().await?;
client.reinitialize().await?;        // re-run the handshake over the live channel
```

`reinitialize` is the only one that refreshes the handshake data;
`supported_models()`, `capabilities()`, and friends read the copy retained at connect
without a round-trip.

## Workspace and tasks

```rust
let file = client.read_file(path, Some(256), None).await?;
```

Reading through the binary — rather than with `std::fs` — is what makes a later edit of
the same path acceptable to the agent's own read-tracking. `max_bytes` truncates
(`ReadFileResult::truncated` says whether it did) and `encoding` requests e.g. base64.
`seed_read_state(path, mtime)` does the same job without transferring contents: it marks
a file as read so a subsequent edit is not rejected as unread.

```rust
client.background_tasks(None).await?;        // move foreground tool calls to the background
client.stop_task("task_0").await?;           // stop one running task
client.rewind_files(user_message_uuid, Some(true)).await?;  // dry-run a file-state rewind
```

`background_tasks(Some(id))` targets one tool call and reports whether it was
backgrounded; `None` backgrounds them all and reports nothing, which counts as success.

`rewind_files` wants the uuid of a **user** message in this session. A dry run reports
what would change (`files_changed`, `insertions`, `deletions`); a real rewind reports
`skipped_links`. Given a uuid the session does not know, the binary answers
`can_rewind: false` with a reason — which is what the example shows.
