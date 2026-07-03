---
type: Rust Trait
title: clauders::agent::runtime::Runtime
description: Runtime — the single trait seam of the Agent SDK core; drives one agent session (send/stream a prompt, issue control operations, expose negotiated capabilities).
tags: [rust, sdk, agent, trait, abstraction]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/runtime.rs
---

Everything above this trait — [Client](/crates/clauders/agent/client.md) —
is concrete and generic over it. Two implementors exist: the
subprocess-backed [CliRuntime](/crates/clauders/agent/cli/runtime.md)
(default) and [MockRuntime](/crates/clauders/agent/mock.md) (test double).

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
}
```

Object-safe (`async_trait` boxes futures), so `&dyn Runtime` compiles.

Related: [Client<R: Runtime>](/crates/clauders/agent/client.md),
[CliRuntime](/crates/clauders/agent/cli/runtime.md),
[MockRuntime](/crates/clauders/agent/mock.md),
[MessageStream](/crates/clauders/agent/stream.md),
[Capabilities](/crates/clauders/agent/capabilities.md).

# Citations

1. `crates/clauders/src/agent/runtime.rs`
