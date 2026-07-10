---
type: Rust Module
title: clauders::agent::runtime::openrouter::tools
description: declare / dispatch — bridge the in-process SdkMcpRegistry to the OpenRouter function-tool surface, namespacing every registered tool as a function tool and routing model tool calls back to the owning server as tool-role messages.
tags: [rust, sdk, agent, runtime, openrouter, mcp, tools]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/openrouter/tools.rs
---

# Schema

```rust
pub(super) fn declare(registry: &SdkMcpRegistry) -> Vec<OrTool>;
pub(super) async fn dispatch(registry: &SdkMcpRegistry, call: &ToolCall) -> OrMessage;
```

`declare` namespaces every registered tool via `declare_name(server,
tool)` (`agent::mcp::naming`, same helper the `api` bridge uses) and emits
an OpenRouter `Tool::function(FunctionDef { name, description, parameters,
.. })`. A name that fails `FunctionName::new` is skipped (not possible for
non-empty server/tool names in practice).

`dispatch` differs from [`api::tools::dispatch`](/crates/clauders/agent/runtime/api/tools.md)
in shape (returns an `OrMessage::tool_result`, a `tool`-role chat message
keyed on the call id, rather than a `ToolResultBlock`) and in one extra
failure mode: OpenRouter tool-call arguments arrive as a JSON-encoded
string (`call.function.arguments`) that must itself be parsed, so a bad
JSON parse is a fourth model-visible error case alongside an unroutable
name, an unknown server, and an unknown tool. Every case — including a
handler `Err` — becomes error message text on the `tool`-role reply, never
a session failure.

# Examples

```rust,no_run
# async fn example() {
use clauders::agent::mcp::{SdkMcpRegistry, SdkMcpServer};
let mut registry = SdkMcpRegistry::default();
registry.register(SdkMcpServer::builder("calc").build());
// declare(&registry) -> one OrTool named "mcp__calc__<tool>" per registered tool
# }
```

Related: [OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md)
(calls `declare` once per turn, `dispatch` once per tool call on a
`tool_calls` finish reason),
[api::tools (structural twin)](/crates/clauders/agent/runtime/api/tools.md),
[Dispatcher (mcp_message control-protocol path)](/crates/clauders/agent/cli/dispatch.md).
The in-process `SdkMcpRegistry`/`SdkMcpServer` module (`agent::mcp`) itself
is out of this scope's coverage; no bundle concept exists for it yet.

# Citations

1. `crates/clauders/src/agent/runtime/openrouter/tools.rs`
