---
type: Rust Trait
title: clauders::agent::runtime::port::Runtime
description: Runtime — the single trait seam of the Agent SDK core; drives one agent session (send/stream a prompt, issue control operations, expose negotiated capabilities and, optionally, a fixed routing identity).
tags: [rust, sdk, agent, trait, abstraction]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/port.rs
---

The trait definition itself moved from the single file `agent/runtime.rs`
into `agent/runtime/port.rs` when the runtime layer regrouped into a
directory of adapters (`api`, `cli`, `openrouter`, `routing`, `mock`) plus
this `port` module; `agent/runtime/mod.rs` re-exports `port::Runtime` as
`Runtime` and aggregates the adapters — see the
[runtime layer overview](/crates/clauders/agent/runtime/overview.md) for
that map. This concept covers the trait itself, unchanged in bundle
location (`agent/runtime.md`) even though its source moved.

Everything above this trait — [Client](/crates/clauders/agent/client.md) —
is concrete and generic over it. Implementors: the subprocess-backed
[CliRuntime](/crates/clauders/agent/cli/runtime.md) (default), the native
[ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) (Messages API),
the native [OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md)
(OpenRouter chat-completions), the meta-adapter
[RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md) (routes
each turn to one of several backend runtimes), and
[MockRuntime](/crates/clauders/agent/mock.md) (test double).

# Schema

```rust
#[async_trait]
pub trait Runtime: Send + Sync {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError>;
    async fn interrupt(&self) -> Result<(), AgentError>;
    async fn set_model(&self, model: ModelId) -> Result<(), AgentError>;
    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError>;
    async fn mcp_status(&self) -> Result<McpStatus, AgentError>;
    fn capabilities(&self) -> &Capabilities;

    /// The stable model identity this runtime was constructed with, if any.
    /// Defaults to `None`.
    fn model(&self) -> Option<&ModelId> {
        None
    }
}
```

Object-safe (`async_trait` boxes futures), so `&dyn Runtime` compiles —
required by [RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md),
which holds its backend targets as `Arc<dyn Runtime>`.

`model()` is new since the prior snapshot: a default-provided method (no
existing implementor is forced to override it) that exposes a fixed
routing identity. `CliRuntime` does not override it (`None` — the CLI
binary can switch models live via `set_model`, so it has no single fixed
identity). `ApiRuntime` and `OpenRouterRuntime` both override it to return
`Some(&ModelId)`, the model they were constructed with — this is exactly
the identity [`RoutingRuntimeBuilder`](/crates/clauders/agent/runtime/routing/builder.md)
reads (via `Runtime::model().ok_or(RoutingError::MissingModelId)`) to key
each target in the routing catalog.

# Citations

1. `crates/clauders/src/agent/runtime/port.rs`
2. `crates/clauders/src/agent/runtime/mod.rs`
