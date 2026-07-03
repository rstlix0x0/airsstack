---
type: Rust Module
title: clauders::agent::client
description: Client<R> — the stateful agent session handle over a Runtime; sends prompts, streams turns, and issues live control operations (interrupt, set_model, set_permission_mode, mcp_status).
tags: [rust, sdk, agent, client, session]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/client.rs
---

Concrete and generic over [Runtime](/crates/clauders/agent/runtime.md),
defaulting to the subprocess-backed
[CliRuntime](/crates/clauders/agent/cli/runtime.md).

# Schema

```rust
pub struct Client<R: Runtime = CliRuntime> { runtime: R }

impl<R: Runtime> Client<R> {
    pub const fn with_runtime(runtime: R) -> Self;
    pub const fn runtime(&self) -> &R;
    pub async fn query(&self, prompt: impl Into<Prompt>) -> Result<MessageStream, AgentError>;
    pub async fn interrupt(&self) -> Result<(), AgentError>;
    pub async fn set_model(&self, model: ModelId) -> Result<(), AgentError>;
    pub async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError>;
    pub async fn mcp_status(&self) -> Result<McpStatus, AgentError>;
    pub fn capabilities(&self) -> &Capabilities;
}

impl Client<CliRuntime> {
    pub fn builder() -> AgentClientBuilder;
    pub async fn connect(options: Options) -> Result<Self, AgentError>;
}

pub struct AgentClientBuilder { options: Options }
```

`query(prompt, options) -> Result<MessageStream, AgentError>` — free
function; sugar over `Client::connect` + `Client::query` whose returned
stream owns the client (session stays alive for the stream's lifetime, torn
down on drop) via an internal `OwningStream` adapter.

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::agent::AgentError> {
use clauders::agent::{Client, Options};
let client = Client::builder().options(Options::default()).connect().await?;
let mut stream = client.query("hi").await?;
# Ok(()) }
```

Related: [Runtime](/crates/clauders/agent/runtime.md),
[CliRuntime::connect](/crates/clauders/agent/cli/runtime.md),
[Options](/crates/clauders/agent/options.md),
[MessageStream](/crates/clauders/agent/stream.md),
[MockRuntime](/crates/clauders/agent/mock.md) (used in `Client` unit tests).

# Citations

1. `crates/clauders/src/agent/client.rs`
