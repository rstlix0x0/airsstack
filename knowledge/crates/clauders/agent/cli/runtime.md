---
type: Rust Module
title: clauders::agent::cli::runtime::CliRuntime
description: The subprocess-backed Runtime implementation — orchestrates discovery, spawn, handshake, a single writer task owning stdin, and a background reader that dispatches control requests and demultiplexes everything else.
tags: [rust, sdk, agent, cli, runtime, subprocess]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/cli/runtime.rs
---

`CliRuntime` composes every other `cli` submodule plus
[process](/crates/clauders/agent/process/overview.md) and
[protocol](/crates/clauders/agent/protocol/overview.md) into a single
[Runtime](/crates/clauders/agent/runtime.md) implementor.

# Schema

```rust
pub struct CliRuntime {
    out_tx: mpsc::UnboundedSender<String>, // single writer task owns stdin
    demux: Arc<Demux>,
    id_gen: RequestIdGen,
    capabilities: Capabilities,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
    _process: ManagedProcess,
}

impl CliRuntime {
    pub async fn connect(options: Options) -> Result<Self, AgentError>;
    async fn send_control(&self, body: OutboundRequestBody, method: &str) -> Result<serde_json::Value, AgentError>;
}
```

`connect` sequence: `discovery::discover` → optional `--version` probe +
`discovery::check_version` → `ManagedProcess::spawn` (via
[process::spawn](/crates/clauders/agent/process/spawn.md)) → handshake
(sends `handshake::initialize_request`, reads until the correlated control
response, parses capabilities) → `handshake::warn_unsupported_hooks` →
spawn the single `writer_loop` task (owns stdin from here on) → build a
[Dispatcher](/crates/clauders/agent/cli/dispatch.md) from `options.hooks`/`permission_policy`
→ spawn `reader_loop` (owns stdout; decodes each line, spawns a Dispatcher
task per inbound control request so a slow handler never stalls the reader,
routes everything else through [Demux](/crates/clauders/agent/cli/demux.md)).

`Runtime` impl: `run` installs a fresh turn sink and writes a user-message
frame; `interrupt`/`set_model`/`set_permission_mode`/`mcp_status` all go
through `send_control`, which registers a `oneshot` waiter keyed by a minted
`RequestId` before sending. `Drop` aborts both background tasks; the owned
`ManagedProcess`'s own `Drop` tears the child down.

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::agent::AgentError> {
use clauders::agent::{CliRuntime, Options};
let runtime = CliRuntime::connect(Options::default()).await?;
# Ok(()) }
```

Related: [Runtime trait](/crates/clauders/agent/runtime.md),
[Client::connect](/crates/clauders/agent/client.md),
[discovery](/crates/clauders/agent/cli/discovery.md),
[argv::build_argv](/crates/clauders/agent/cli/argv.md),
[handshake](/crates/clauders/agent/cli/handshake.md),
[Demux](/crates/clauders/agent/cli/demux.md),
[Dispatcher](/crates/clauders/agent/cli/dispatch.md),
[ManagedProcess](/crates/clauders/agent/process/handle.md),
[protocol codec](/crates/clauders/agent/protocol/codec.md).

# Citations

1. `crates/clauders/src/agent/cli/runtime.rs`
