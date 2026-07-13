---
type: Rust Module
title: clauders::agent::runtime::mock
description: MockRuntime — an in-memory Runtime test double with no subprocess, replaying scripted message turns, recording control operations for test assertions, and optionally reporting a fixed routing identity.
tags: [rust, sdk, agent, testing, mock]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/mock.rs
---

Relocated from `agent/mock.rs` to `agent/runtime/mock.rs` (`#[cfg(test)]
pub mod mock` inside the regrouped runtime layer) — see the
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).
Available to downstream crates through the `__test-mocks` feature,
mirroring the crate's mock HTTP transport
([types/mod.rs](/crates/clauders/types/api-key.md) area) used by the
non-agent SDK.

# Schema

```rust
pub enum ControlCall { Interrupt, SetModel(ModelId), SetPermissionMode(PermissionMode), McpStatus }

pub struct MockRuntime {
    scripts: Mutex<VecDeque<Vec<Message>>>,
    calls: Mutex<Vec<ControlCall>>,
    capabilities: Capabilities,
    mcp_status: McpStatus,
    model: Option<ModelId>,
}
```

`MockRuntime::new(scripts: Vec<Vec<Message>>)` — replays one queued turn per
`run` call (empty script when exhausted). `with_capabilities`,
`with_mcp_status`, `with_model(ModelId)` — builder-style overrides;
`with_model` is new since the prior snapshot, setting the identity
`model()` reports (defaults to `None`). `calls() -> Vec<ControlCall>` —
recorded control operations in call order. Implements
[Runtime](/crates/clauders/agent/runtime.md): every control method records
a `ControlCall` and returns `Ok`; `model()` returns `self.model.as_ref()`.

`with_model` is what lets tests build a routing catalog of mocks keyed by
identity — see
[RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md)'s and
[RoutingRuntimeBuilder](/crates/clauders/agent/runtime/routing/builder.md)'s
own test suites, which construct `MockRuntime::new(..).with_model(id)`
targets exactly this way.

# Examples

```rust
use clauders::agent::{Client, MockRuntime};
let client = Client::with_runtime(MockRuntime::new(vec![vec![/* scripted Message::Result */]]));
```

```rust,no_run
# fn example() {
use clauders::agent::MockRuntime;
use clauders::types::ModelId;
// A mock with a fixed routing identity, usable as a RoutingRuntime target:
let _mock = MockRuntime::new(vec![]).with_model(ModelId::custom("deepseek/deepseek-chat").unwrap());
# }
```

Related: [Runtime trait](/crates/clauders/agent/runtime.md),
[Client (used in tests via with_runtime)](/crates/clauders/agent/client.md),
[MessageStream](/crates/clauders/agent/stream.md) (`ReceiverStream` backs
the mock's replay channel),
[RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md) (test
suite uses `with_model` mocks as targets),
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Citations

1. `crates/clauders/src/agent/runtime/mock.rs`
