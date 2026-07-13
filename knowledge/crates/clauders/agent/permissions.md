---
type: Rust Module
title: clauders::agent::permissions
description: Permission control for the agent — PermissionMode (incl. DontAsk), PermissionContext/PermissionDecision (interrupt + updated_permissions + constructors), and the PermissionPolicy trait consulted by the runtime's in-loop handler.
tags: [rust, sdk, agent, permissions, security]
timestamp: 2026-07-11T00:00:00Z
resource: crates/clauders/src/agent/permissions/mod.rs
---

`crates/clauders/src/agent/permissions.rs` was split into an export-only
folder module: `mode.rs` (`PermissionMode`), `decision.rs`
(`PermissionContext`/`PermissionDecision`), `policy.rs`
(`PermissionPolicy`), and `update.rs` (the rule-update cluster, its own
concept — see below). `mod.rs` itself only re-exports; it declares no
new types.

# Schema

```rust
pub enum PermissionMode { // wire: camelCase; forwarded via set_permission_mode control request
    Default,             // "default", #[default]
    AcceptEdits,          // "acceptEdits"
    Plan,                  // "plan"
    BypassPermissions,     // "bypassPermissions"
    DontAsk,                // "dontAsk" — deny any tool not pre-approved, without prompting
}

pub struct PermissionContext {
    pub tool_use_id: Option<String>,
    pub agent_id: Option<String>,
    pub blocked_path: Option<String>,
    pub decision_reason: Option<String>,
    pub title: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

pub enum PermissionDecision {
    Allow {
        updated_input: Option<serde_json::Value>,
        updated_permissions: Vec<PermissionUpdate>,
    },
    Deny {
        message: String,
        interrupt: bool,               // true aborts the whole turn, not just this call
        updated_permissions: Vec<PermissionUpdate>,
    },
}

impl PermissionDecision {
    pub const fn allow() -> Self;                                  // Allow, no rewrite, no updates
    pub const fn allow_with(input: serde_json::Value) -> Self;      // Allow, rewritten input, no updates
    pub fn deny(message: impl Into<String>) -> Self;                // Deny, interrupt: false, no updates
    pub fn deny_interrupt(message: impl Into<String>) -> Self;      // Deny, interrupt: true, no updates
    pub fn updated_permissions(&self) -> &[PermissionUpdate];       // reads either variant
    pub fn into_response_value(self, original_input: &serde_json::Value) -> serde_json::Value;
}

#[async_trait]
pub trait PermissionPolicy: Send + Sync {
    async fn can_use_tool(&self, tool: &str, input: &serde_json::Value, ctx: PermissionContext)
        -> Result<PermissionDecision, AgentError>;
}
```

`PermissionDecision::into_response_value(original_input)` renders the
binary's wire shape: `{"behavior":"allow","updatedInput":…}` (echoing
`original_input` when no rewrite is supplied, via `updated_input`) or
`{"behavior":"deny","message":…,"interrupt":…}`. On either variant, a
non-empty `updated_permissions` is attached as an `updatedPermissions`
array (camelCase-serialized [`PermissionUpdate`](/crates/clauders/agent/permissions/update.md)
values); an empty one is omitted from the payload entirely rather than
serialized as `[]`.

The four constructors (`allow`, `allow_with`, `deny`, `deny_interrupt`)
are the idiomatic way to build a decision without naming every field —
all four default `updated_permissions` to empty; a caller who needs to
attach rule updates constructs the enum variant directly (as
[`permission_engine::evaluate`](/crates/clauders/agent/runtime/permission_engine.md)
and its native rule-store fold do internally).

# Examples

```rust
use clauders::agent::PermissionMode;
assert_eq!(PermissionMode::default(), PermissionMode::Default);
let json = serde_json::to_string(&PermissionMode::AcceptEdits).unwrap();
assert_eq!(json, "\"acceptEdits\"");
let json = serde_json::to_string(&PermissionMode::DontAsk).unwrap();
assert_eq!(json, "\"dontAsk\"");
```

```rust
use clauders::agent::PermissionDecision;
let original = serde_json::json!({ "cmd": "ls" });
let value = PermissionDecision::allow().into_response_value(&original);
assert_eq!(value["behavior"], "allow");
assert_eq!(value["updatedInput"], serde_json::json!({ "cmd": "ls" }));
assert!(value.get("updatedPermissions").is_none());

let value = PermissionDecision::deny_interrupt("stop").into_response_value(&serde_json::json!({}));
assert_eq!(value["interrupt"], true);
```

Related: [PermissionUpdate/PermissionBehavior/PermissionScope](/crates/clauders/agent/permissions/update.md)
(the rule-update cluster carried by `updated_permissions`),
[permission_engine (RuleStore/evaluate)](/crates/clauders/agent/runtime/permission_engine.md)
(the native enforcement engine that consults `PermissionMode` and a
`PermissionPolicy` to produce the canonical `PermissionDecision`),
[ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) (gates every
tool call through `permission_engine::evaluate` and interrupts the turn
on `Deny { interrupt: true, .. }`),
[Options::permission_mode/permission_policy](/crates/clauders/agent/options.md),
[Dispatcher::permission_outcome](/crates/clauders/agent/cli/dispatch.md)
(consults the registered `PermissionPolicy` on the CLI-subprocess path),
[cli::argv::permission_mode_wire](/crates/clauders/agent/cli/argv.md),
[Client::set_permission_mode](/crates/clauders/agent/client.md).

# Citations

1. `crates/clauders/src/agent/permissions/mod.rs`
2. `crates/clauders/src/agent/permissions/mode.rs`
3. `crates/clauders/src/agent/permissions/decision.rs`
4. `crates/clauders/src/agent/permissions/policy.rs`
