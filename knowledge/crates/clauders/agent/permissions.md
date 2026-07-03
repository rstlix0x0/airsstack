---
type: Rust Module
title: clauders::agent::permissions
description: Permission control for the agent — PermissionMode (forwarded to the binary), PermissionContext/PermissionDecision, and the PermissionPolicy trait consulted by the runtime's in-loop handler.
tags: [rust, sdk, agent, permissions, security]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/permissions.rs
---

# Schema

```rust
pub enum PermissionMode { // wire: camelCase; forwarded via set_permission_mode control request
    Default,           // "default", #[default]
    AcceptEdits,        // "acceptEdits"
    Plan,                // "plan"
    BypassPermissions,   // "bypassPermissions"
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
    Allow { updated_input: Option<serde_json::Value> },
    Deny { message: String },
}

#[async_trait]
pub trait PermissionPolicy: Send + Sync {
    async fn can_use_tool(&self, tool: &str, input: &serde_json::Value, ctx: PermissionContext)
        -> Result<PermissionDecision, AgentError>;
}
```

`PermissionDecision::into_response_value(original_input) -> serde_json::Value`
renders the binary's wire shape: `{"behavior":"allow","updatedInput":…}`
(echoing `original_input` when no rewrite is supplied) or
`{"behavior":"deny","message":…}`.

# Examples

```rust
use clauders::agent::PermissionMode;
assert_eq!(PermissionMode::default(), PermissionMode::Default);
let json = serde_json::to_string(&PermissionMode::AcceptEdits).unwrap();
assert_eq!(json, "\"acceptEdits\"");
```

Related: [Options::permission_mode/permission_policy](/crates/clauders/agent/options.md),
[Dispatcher::permission_outcome](/crates/clauders/agent/cli/dispatch.md)
(consults the registered `PermissionPolicy`),
[cli::argv::permission_mode_wire](/crates/clauders/agent/cli/argv.md),
[Client::set_permission_mode](/crates/clauders/agent/client.md).

# Citations

1. `crates/clauders/src/agent/permissions.rs`
