---
type: Rust Module
title: clauders::agent::process::error::ProcessError
description: ProcessError — the closed set of subprocess-management failure modes (spawn, kill, timeout, already-shut-down), Clone so the supervisor can publish one outcome to multiple awaiters.
tags: [rust, sdk, agent, process, error-handling]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/process/error.rs
---

I/O error payloads are captured as their formatted string so the error type
stays `Clone` — the [Supervisor](/crates/clauders/agent/process/supervisor.md)
publishes its outcome to multiple awaiters through a `watch` channel.

# Schema

```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProcessError {
    Spawn(String),         // the child process could not be spawned
    Kill(String),          // killing the child (or group) failed
    Timeout,               // did not exit within the shutdown grace period even after a forced kill
    AlreadyShutDown,       // already torn down; no exit status available
}
```

Related: [AgentError::Process](/crates/clauders/agent/error.md) (wraps this
type via `#[from]`), [ManagedProcess::spawn/shutdown/wait](/crates/clauders/agent/process/handle.md),
[Supervisor::run](/crates/clauders/agent/process/supervisor.md).

# Citations

1. `crates/clauders/src/agent/process/error.rs`
