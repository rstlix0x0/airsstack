# 06 — Hooks

Run Rust code at fixed points in the agent's loop, and veto a step when you don't
like the look of it.

## Run

```text
cargo run -p clauders --example agent_06_hooks
```

The agent runs `pwd` (traced, allowed) and then tries `rm -rf ./scratch` (blocked by
a hook before the shell sees it).

## What it shows

```rust
#[async_trait::async_trait]
impl Hook for Trace {
    async fn call(&self, input: HookInput, _cancel: CancelSignal)
        -> Result<HookOutput, AgentError>
    {
        println!("{:?} {:?}", input.event, input.tool_use_id);
        Ok(HookOutput::default())
    }
}

let options = Options::builder()
    .hook(HookEvent::PreToolUse, Some("Bash".to_owned()), Arc::new(BlockDangerousBash))
    .hook(HookEvent::PreToolUse, None, Arc::new(Trace { label: "pre" }))
    .hook(HookEvent::Stop, None, Arc::new(Trace { label: "stop" }))
    .include_hook_events(true)
    .build();
```

Register as many as you like. The second argument is a **matcher**: `Some("Bash")`
fires only for that tool, `None` fires for every one. Registration order decides the
callback id the binary echoes back, so the same handler type can be registered
several times and stay distinguishable.

## The events

`HookEvent` covers the loop, not just tools:

| Event | Fires |
|---|---|
| `PreToolUse` / `PostToolUse` / `PostToolUseFailure` | around each tool call |
| `UserPromptSubmit` | when a user turn is submitted |
| `Stop` | when the turn stops |
| `SubagentStart` / `SubagentStop` | around a delegated subagent |
| `PreCompact` | before context compaction |
| `Notification` | on a binary notification |
| `PermissionRequest` | when a permission request is raised |
| `SessionStart` / `SessionEnd` / `Setup` | session lifecycle |
| `Elicitation` / `ElicitationResult` | around an MCP elicitation |

Registering an event this binary never fires is harmless — it simply never fires.

## What a hook can return

```rust
HookOutput {
    decision: Some(HookDecision::Block),
    reason: Some("`rm -rf` is on this session's denylist".to_owned()),
    system_message: Some("A hook blocked that. Suggest a read-only alternative.".to_owned()),
    ..HookOutput::default()
}
```

| Field | Effect |
|---|---|
| `decision: Some(HookDecision::Block)` | veto the step the hook fired on |
| `reason` | human-readable explanation of the decision |
| `system_message` | injected into the conversation, so the model can react |
| `continue_: Some(false)` | stop the agent loop |
| `suppress_output: Some(true)` | hide the binary's own output for this step |

`HookOutput::default()` is an empty object: observe and get out of the way.

## Reading the event payload

```rust
let command = input.data
    .get("tool_input")
    .and_then(|tool_input| tool_input.get("command"))
    .and_then(serde_json::Value::as_str)
    .unwrap_or_default();
```

`HookInput::data` is the binary's own event body, opaque to the SDK — its shape
varies by event, so it is passed through rather than typed. `input.event` and
`input.tool_use_id` are typed.

## Hooks versus permission policies

They are different levers, and using both is normal:

- a `PermissionPolicy` (example 05) answers exactly one question — *may this tool
  run, with these arguments* — and can rewrite the arguments;
- a `Hook` runs *around* a step, can fire on events that have nothing to do with
  tools, and can inject a message into the conversation.

## Observing the hooks themselves

```rust
.include_hook_events(true)
```

Asks the binary to emit hook-lifecycle frames onto the message stream. This SDK does
not model their shape, so they arrive as `Message::Other` with their JSON intact:

```rust
Message::Other(raw) => {
    if let Some(kind) = raw.get("type").and_then(serde_json::Value::as_str) {
        println!("[lifecycle frame] {kind}");
    }
}
```
