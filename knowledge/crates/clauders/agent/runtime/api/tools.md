---
type: Rust Module
title: clauders::agent::runtime::api::tools
description: declare / dispatch — bridge the in-process SdkMcpRegistry to the Messages API tool surface, namespacing every registered tool as a wire Tool and routing model tool calls back to the owning server.
tags: [rust, sdk, agent, runtime, messages-api, mcp, tools]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/api/tools.rs
---

# Schema

```rust
pub(super) fn declare(registry: &SdkMcpRegistry) -> Vec<WireTool>;
pub(super) async fn dispatch(registry: &SdkMcpRegistry, block: &ToolUseBlock) -> ToolResultBlock;
```

`declare` walks every server and tool in the registry, namespacing each
name via `declare_name(server, tool)` (re-exported from `agent::mcp::naming`
through [`convert`](/crates/clauders/agent/runtime/api/convert.md)) and
emitting a `messages::tools::Tool` with the tool's description and input
schema. A namespaced name that fails `ToolName::new` (empty; not possible
for non-empty server/tool names in practice) is skipped rather than
panicking.

`dispatch` routes one model tool-use block back to its owning server:
parses the namespaced tool name via `route(name)`, looks the server and
tool up in the registry, calls the tool, and shapes the outcome as a
`ToolResultBlock`. Every failure mode — an unroutable name, an unknown
server, an unknown tool, or a handler error — becomes a model-visible
error result (`ToolResultBlock::err`), never a session failure, matching
the contract of the crate's own JSON-RPC MCP router used by
[`Dispatcher::mcp_outcome`](/crates/clauders/agent/cli/dispatch.md).

# Examples

```rust,no_run
# async fn example() {
use clauders::agent::mcp::{SdkMcpRegistry, SdkMcpServer};
let mut registry = SdkMcpRegistry::default();
registry.register(SdkMcpServer::builder("calc").build());
// declare(&registry) -> one WireTool named "mcp__calc__<tool>" per registered tool
# }
```

Related: [ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) (calls
`declare` once per turn to build the tool set, and `dispatch` per
`ToolUse` block on a `tool_use` stop reason),
[convert (declare_name/route re-export)](/crates/clauders/agent/runtime/api/convert.md),
[messages::tools](/crates/clauders/messages/tools.md),
[openrouter::tools](/crates/clauders/agent/runtime/openrouter/tools.md)
(the structural twin for OpenRouter function-calling),
[Dispatcher (mcp_message control-protocol path)](/crates/clauders/agent/cli/dispatch.md).
The in-process `SdkMcpRegistry`/`SdkMcpServer` module (`agent::mcp`) itself
is out of this scope's coverage; no bundle concept exists for it yet.

# Citations

1. `crates/clauders/src/agent/runtime/api/tools.rs`
