---
type: Rust Module
title: clauders::agent::capabilities
description: Capabilities — the feature manifest negotiated with the claude binary during the initialize handshake, and HookEvent, the closed set of hookable lifecycle events.
tags: [rust, sdk, agent, capabilities, handshake]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/capabilities.rs
---

Used to gate optional features and degrade gracefully across binary
versions: a feature absent from the manifest reads as unsupported rather
than assumed present.

# Schema

```rust
pub enum HookEvent { // wire names are PascalCase as the binary emits them
    PreToolUse, PostToolUse, PostToolUseFailure, UserPromptSubmit, Stop,
    SubagentStart, SubagentStop, PreCompact, Notification, PermissionRequest,
}

pub struct Capabilities {
    pub protocol_version: String,
    pub supported_hook_events: HashSet<HookEvent>,
    pub supported_control_methods: HashSet<String>,
}
```

`Capabilities::supports_hook(HookEvent) -> bool`,
`supports_control(&str) -> bool`. Missing fields default to empty
(`Default` impl), so an unrecognized or malformed handshake response yields
an "unsupported" reading rather than a hard failure.

# Examples

```rust
use clauders::agent::{Capabilities, HookEvent};
let caps: Capabilities = serde_json::from_str(r#"{"protocol_version":"1.0","supported_hook_events":["PreToolUse","Stop"],"supported_control_methods":["interrupt"]}"#).unwrap();
assert!(caps.supports_hook(HookEvent::PreToolUse));
```

Related: [handshake::parse_capabilities](/crates/clauders/agent/cli/handshake.md),
[HookRegistry::initialize_payload](/crates/clauders/agent/hooks.md),
[Client::capabilities](/crates/clauders/agent/client.md),
[Runtime::capabilities](/crates/clauders/agent/runtime.md).

# Citations

1. `crates/clauders/src/agent/capabilities.rs`
