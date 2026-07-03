---
type: Rust Module
title: clauders::agent::mock
description: MockRuntime — an in-memory Runtime test double with no subprocess, replaying scripted message turns and recording control operations for test assertions.
tags: [rust, sdk, agent, testing, mock]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/mock.rs
---

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
}
```

`MockRuntime::new(scripts: Vec<Vec<Message>>)` — replays one queued turn per
`run` call (empty script when exhausted). `with_capabilities`,
`with_mcp_status` — builder-style overrides. `calls() -> Vec<ControlCall>` —
recorded control operations in call order. Implements
[Runtime](/crates/clauders/agent/runtime.md): every control method records
a `ControlCall` and returns `Ok`.

# Examples

```rust
use clauders::agent::{Client, MockRuntime};
let client = Client::with_runtime(MockRuntime::new(vec![vec![/* scripted Message::Result */]]));
```

Related: [Runtime trait](/crates/clauders/agent/runtime.md),
[Client (used in tests via with_runtime)](/crates/clauders/agent/client.md),
[MessageStream](/crates/clauders/agent/stream.md) (`ReceiverStream` backs
the mock's replay channel).

# Citations

1. `crates/clauders/src/agent/mock.rs`
