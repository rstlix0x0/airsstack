# 05 — Permissions

Decide each tool call in Rust: allow it, rewrite its arguments, or refuse it.

## Run

```text
cargo run -p clauders --example agent_05_permissions
```

The agent is asked to run `ls` (allowed, and rewritten) and then
`rm -rf /tmp/does-not-exist` (refused). Nothing destructive reaches the shell.

## What it shows

With a policy registered, the binary asks *this program* before running a gated
tool:

```rust
#[async_trait::async_trait]
impl PermissionPolicy for ReadOnlyShell {
    async fn can_use_tool(
        &self,
        tool: &str,
        input: &serde_json::Value,
        ctx: PermissionContext,
        cancel: CancelSignal,
    ) -> Result<PermissionDecision, AgentError> { /* … */ }
}

let options = Options::builder()
    .permission_mode(PermissionMode::Default)   // "ask" — this is what routes calls here
    .allowed_tools(vec!["Bash".to_owned()])
    .permission_policy(Arc::new(ReadOnlyShell { seen: AtomicUsize::new(0) }))
    .build();
```

The mode matters: `Default` asks, so the policy is consulted. `BypassPermissions`
skips it entirely.

## The four decisions

```rust
PermissionDecision::allow()                    // run it unchanged
PermissionDecision::allow_with(rewritten)      // run it with different arguments
PermissionDecision::deny("why")                // refuse this call, turn continues
PermissionDecision::deny_interrupt("why")      // refuse and abort the whole turn
```

Rewriting is the interesting one — the model asked for `ls`, the policy substitutes
`ls -la`, and the model sees the output of what actually ran:

```rust
let mut rewritten = input.clone();
rewritten["command"] = serde_json::Value::String("ls -la".to_owned());
return Ok(PermissionDecision::allow_with(rewritten));
```

Use it to force flags, pin a path inside a sandbox, or strip an argument you never
want passed.

## Persisting rules

A decision can carry `PermissionUpdate`s, which is how "always allow this" is
expressed:

```rust
PermissionDecision::Allow {
    updated_input: None,
    updated_permissions: vec![PermissionUpdate::AddRules {
        rules: vec![PermissionRuleValue {
            tool_name: "Bash".to_owned(),
            rule_content: Some(command.to_owned()),
        }],
        behavior: PermissionBehavior::Allow,
        destination: PermissionUpdateDestination::Session,
    }],
}
```

`behavior` is `Allow`, `Deny`, or `Ask`. `destination` decides how long it lasts:
`Session` for this run only, or `UserSettings` / `ProjectSettings` /
`LocalSettings` to write it to a settings file. `PermissionUpdate` also covers
`ReplaceRules`, `RemoveRules`, `SetMode`, `AddDirectories`, and
`RemoveDirectories`.

## What the request tells you

`PermissionContext` carries whatever the binary supplied. `request_id` is always
present; everything else is optional:

- `tool_use_id`, `agent_id` — which call, and which (sub)agent made it.
- `title`, `display_name`, `description`, `decision_reason` — the binary's own
  human-facing framing of the request.
- `blocked_path` — the path the call is blocked on, when that is the reason.
- `matched_ask_rule` — set when a user-configured `permissions.ask` rule forced the
  prompt. A host that auto-approves should treat a request carrying this as
  user-intended and ask anyway.
- `suggestions` — rule updates the binary suggests you might apply. Echoing them
  back as `updated_permissions` round-trips byte-exactly, including any suggestion
  kind this release does not model.

## Cancellation

```rust
if cancel.is_cancelled() {
    return Err(AgentError::Interrupted);
}
```

The binary can withdraw a request while a slow policy is still deciding.
Cancellation is cooperative — nothing kills the task — so a policy that cares must
check the signal (or `await cancel.cancelled()` in a `select!`). Ignoring it is
valid: the policy runs to completion and its answer is still written.

## Reading the outcome

```rust
Message::Result(result) => {
    println!("denials recorded: {}", result.permission_denials.len());
}
```

Every refusal during the turn is recorded on the result frame.
