---
type: Rust Module
title: clauders::agent::permissions::update
description: PermissionUpdate/PermissionBehavior/PermissionScope — the rule-update cluster a PermissionDecision carries in its updated_permissions list, serialized into the CLI's updatedPermissions array for passthrough.
tags: [rust, sdk, agent, permissions, security, rules]
timestamp: 2026-07-11T00:00:00Z
resource: crates/clauders/src/agent/permissions/update.rs
---

Part of the [`agent::permissions` module](/crates/clauders/agent/permissions.md).
Natively, only `tool` + `behavior` are acted on — by
[`permission_engine::RuleStore`](/crates/clauders/agent/runtime/permission_engine.md);
`scope` is carried for CLI wire fidelity only, since the native rule
store treats every update as session-scoped regardless of its declared
`scope`.

`PermissionBehavior` is a bare allow/deny discriminant shared by
`PermissionUpdate` and the native rule store. It is distinct in shape and
role from the payload-carrying
[`PermissionDecision`](/crates/clauders/agent/permissions.md) — not a
duplicate of it: `PermissionDecision` is the verdict on one tool call;
`PermissionBehavior` is the standing rule a decision may ask the runtime
to remember for later calls.

# Schema

```rust
#[serde(rename_all = "camelCase")]
pub struct PermissionUpdate {
    pub behavior: PermissionBehavior,
    pub tool: String,
    pub scope: PermissionScope,
}

#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    Allow,   // "allow"
    Deny,    // "deny"
}

#[serde(rename_all = "lowercase")]
pub enum PermissionScope {
    Session,  // "session" — the only scope the native rule store honors
    Local,    // "local"   — CLI wire fidelity only
    Project,  // "project" — CLI wire fidelity only
    User,     // "user"    — CLI wire fidelity only
}
```

# Examples

```rust
use clauders::agent::{PermissionBehavior, PermissionScope, PermissionUpdate};

let update = PermissionUpdate {
    behavior: PermissionBehavior::Allow,
    tool: "Bash".to_string(),
    scope: PermissionScope::Session,
};
let value = serde_json::to_value(&update).unwrap();
assert_eq!(value["behavior"], "allow");
assert_eq!(value["tool"], "Bash");
assert_eq!(value["scope"], "session");
```

Related: [PermissionDecision::updated_permissions/into_response_value](/crates/clauders/agent/permissions.md)
(the producer of `PermissionUpdate` values, and the point where a
non-empty list is rendered as the wire's `updatedPermissions` array),
[permission_engine::RuleStore::apply](/crates/clauders/agent/runtime/permission_engine.md)
(the native consumer — folds `behavior`/`tool` into the session-scoped
store, ignoring `scope`).

# Citations

1. `crates/clauders/src/agent/permissions/update.rs`
