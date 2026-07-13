---
type: Rust Module
title: clauders::agent::runtime::cli::dispatch
description: Dispatcher — answers inbound control requests (can_use_tool, hook_callback, mcp_message) by consulting the registered PermissionPolicy, Hook, or in-process SdkMcpRegistry, and enqueuing a correlated control response.
tags: [rust, sdk, agent, cli, dispatch, control-protocol, mcp]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/cli/dispatch.rs
---

Relocated from `agent/cli/dispatch.rs` to `agent/runtime/cli/dispatch.rs`
in the runtime-adapter regroup — see the
[runtime layer overview](/crates/clauders/agent/runtime/overview.md). The
`Dispatcher` also gained a third inbound-request kind since the prior
snapshot: `mcp_message`, routing in-process MCP tool calls declared via
`Options.sdk_mcp_servers`.

The reader task intercepts each inbound `control_request` and hands it to a
`Dispatcher`. A handler error becomes an error control response so the
binary is never left waiting.

# Schema

```rust
pub(super) struct Dispatcher {
    hooks: Arc<HookRegistry>,
    policy: Option<Arc<dyn PermissionPolicy>>,
    mcp: Arc<SdkMcpRegistry>,
    out_tx: mpsc::UnboundedSender<String>,
}

impl Dispatcher {
    pub(super) fn new(
        hooks: Arc<HookRegistry>,
        policy: Option<Arc<dyn PermissionPolicy>>,
        mcp: Arc<SdkMcpRegistry>,
        out_tx: mpsc::UnboundedSender<String>,
    ) -> Self;
    pub(super) async fn handle(&self, req: InboundControlRequest);
    async fn permission_outcome(&self, tool: &str, input: serde_json::Value, ctx: PermissionContext) -> Result<serde_json::Value, String>;
    async fn hook_outcome(&self, callback_id: &str, data: serde_json::Value, tool_use_id: Option<String>) -> Result<serde_json::Value, String>;
    async fn mcp_outcome(&self, server_name: &str, message: serde_json::Value) -> Result<serde_json::Value, String>;
    fn write_response(&self, request_id: String, outcome: Result<serde_json::Value, String>);
}
```

`handle` dispatches on `InboundRequestBody`: `CanUseTool` →
`permission_outcome`, `HookCallback` → `hook_outcome`, `McpMessage` →
`mcp_outcome`.

`permission_outcome`: with no registered policy, allows and echoes the
original input (`PermissionDecision::Allow { updated_input: None }`); with
a policy, delegates to `PermissionPolicy::can_use_tool` and renders the
decision via `into_response_value`. `hook_outcome`: unknown callback id →
empty-object success (never hangs the binary); registered hook's `Err` →
error control response carrying the handler's message. `mcp_outcome`:
looks up `server_name` in the `SdkMcpRegistry`; on a hit, hands the raw
JSON-RPC `message` to `mcp::router::handle_mcp_message` and wraps the
reply as `{"mcp_response": ..}`; on a miss, an error control response
naming the unknown server.

# Examples

```rust,no_run
# use std::sync::Arc;
# use tokio::sync::mpsc;
use clauders::agent::hooks::HookRegistry;
let (tx, _rx) = mpsc::unbounded_channel();
// Dispatcher::new(Arc::new(HookRegistry::default()), None, Arc::new(Default::default()), tx);
```

Related: [PermissionPolicy / PermissionDecision](/crates/clauders/agent/permissions.md),
[HookRegistry / Hook](/crates/clauders/agent/hooks.md),
[protocol frames (InboundControlRequest/OutboundControlResponse)](/crates/clauders/agent/protocol/frames.md),
[CliRuntime](/crates/clauders/agent/cli/runtime.md) (spawns a `Dispatcher`
per inbound control request so a slow handler never stalls the reader,
and is the one that builds the `Arc<SdkMcpRegistry>` from
`options.sdk_mcp_servers`),
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).
The in-process `SdkMcpRegistry`/`SdkMcpServer` module (`agent::mcp`) is
out of this scope's coverage; no bundle concept exists for it yet.

# Citations

1. `crates/clauders/src/agent/runtime/cli/dispatch.rs`
