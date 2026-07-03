---
type: Rust Module
title: clauders::agent::hooks
description: In-loop hook handlers and their registry — Hook trait, HookOutput's camelCase wire shape, and HookRegistry, which mints hook_<n> callback ids for the initialize handshake.
tags: [rust, sdk, agent, hooks, extensibility]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/hooks.rs
---

A `Hook` is consulted when the binary fires a `hook_callback` control
request (dispatched by [Dispatcher](/crates/clauders/agent/cli/dispatch.md)).
`HookOutput` is serialized to the binary's camelCase wire shape and returned
as the correlated control response.

# Schema

```rust
pub enum HookDecision { Block } // #[serde(rename_all = "lowercase")]; only variant defined by the protocol today

pub struct HookOutput {
    pub continue_: Option<bool>,       // wire: "continue"
    pub suppress_output: Option<bool>, // wire: "suppressOutput"
    pub decision: Option<HookDecision>,
    pub system_message: Option<String>, // wire: "systemMessage"
    pub reason: Option<String>,
}

pub struct HookInput { pub event: HookEvent, pub tool_use_id: Option<String>, pub data: serde_json::Value }

#[async_trait]
pub trait Hook: Send + Sync {
    async fn call(&self, input: HookInput) -> Result<HookOutput, AgentError>;
}

pub struct HookRegistry { entries: Vec<HookEntry> } // Clone; handlers are Arc-shared
```

`HookRegistry::register(event, matcher, hook: Arc<dyn Hook>) -> &mut Self`
mints a `hook_<index>` callback id in registration order.
`HookRegistry::lookup(callback_id) -> Option<(HookEvent, Arc<dyn Hook>)>`.
`HookRegistry::initialize_payload(&Capabilities) -> serde_json::Value`
groups entries by PascalCase event name into
`{event: [{matcher?, hookCallbackIds:[…]}]}` for the initialize handshake;
when `caps` lists supported events (non-empty), unsupported events are
omitted with a `tracing::warn!`; when `caps` is empty (unknown
pre-handshake), all events are included.

# Examples

```rust
use clauders::agent::{HookRegistry, HookEvent};
let mut reg = HookRegistry::default();
// reg.register(HookEvent::PreToolUse, Some("Bash".into()), Arc::new(my_hook));
assert!(reg.is_empty());
```

Related: [Options::hook / hooks builder method](/crates/clauders/agent/options.md),
[Dispatcher::hook_outcome](/crates/clauders/agent/cli/dispatch.md),
[handshake::initialize_request](/crates/clauders/agent/cli/handshake.md),
[Capabilities](/crates/clauders/agent/capabilities.md).

# Citations

1. `crates/clauders/src/agent/hooks.rs`
