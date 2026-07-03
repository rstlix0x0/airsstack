---
type: Rust Module
title: clauders::agent::cli::dispatch
description: Dispatcher — answers inbound control requests (can_use_tool, hook_callback) by consulting the registered PermissionPolicy or Hook and enqueuing a correlated control response.
tags: [rust, sdk, agent, cli, dispatch, control-protocol]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/cli/dispatch.rs
---

The reader task intercepts each inbound `control_request` and hands it to a
`Dispatcher`. A handler error becomes an error control response so the
binary is never left waiting.

# Schema

```rust
pub(super) struct Dispatcher {
    hooks: Arc<HookRegistry>,
    policy: Option<Arc<dyn PermissionPolicy>>,
    out_tx: mpsc::UnboundedSender<String>,
}

impl Dispatcher {
    pub(super) fn new(hooks: Arc<HookRegistry>, policy: Option<Arc<dyn PermissionPolicy>>, out_tx: mpsc::UnboundedSender<String>) -> Self;
    pub(super) async fn handle(&self, req: InboundControlRequest);
    async fn permission_outcome(&self, tool: &str, input: serde_json::Value, ctx: PermissionContext) -> Result<serde_json::Value, String>;
    async fn hook_outcome(&self, callback_id: &str, data: serde_json::Value, tool_use_id: Option<String>) -> Result<serde_json::Value, String>;
    fn write_response(&self, request_id: String, outcome: Result<serde_json::Value, String>);
}
```

`permission_outcome`: with no registered policy, allows and echoes the
original input (`PermissionDecision::Allow { updated_input: None }`); with
a policy, delegates to `PermissionPolicy::can_use_tool` and renders the
decision via `into_response_value`. `hook_outcome`: unknown callback id →
empty-object success (never hangs the binary); registered hook's `Err` →
error control response carrying the handler's message.

# Examples

```rust,no_run
# use std::sync::Arc;
# use tokio::sync::mpsc;
use clauders::agent::hooks::HookRegistry;
let (tx, _rx) = mpsc::unbounded_channel();
// Dispatcher::new(Arc::new(HookRegistry::default()), None, tx);
```

Related: [PermissionPolicy / PermissionDecision](/crates/clauders/agent/permissions.md),
[HookRegistry / Hook](/crates/clauders/agent/hooks.md),
[protocol frames (InboundControlRequest/OutboundControlResponse)](/crates/clauders/agent/protocol/frames.md),
[CliRuntime](/crates/clauders/agent/cli/runtime.md) (spawns a `Dispatcher`
per inbound control request so a slow handler never stalls the reader).

# Citations

1. `crates/clauders/src/agent/cli/dispatch.rs`
