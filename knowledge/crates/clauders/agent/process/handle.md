---
type: Rust Module
title: clauders::agent::process::handle::ManagedProcess
description: ManagedProcess — the owned handle to a supervised child process; its Drop impl requests teardown so a dropped handle can never orphan the child.
tags: [rust, sdk, agent, process, lifecycle]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/process/handle.rs
---

`Drop` is collocated with the struct/impl block deliberately: `Drop`
accesses the private `supervisor` field, and a sibling file cannot name
private fields of a type defined elsewhere.

# Schema

```rust
pub struct ManagedProcess { supervisor: Arc<Supervisor> }

impl ManagedProcess {
    pub fn spawn(cfg: &ProcessConfig) -> Result<(Self, ProcessIo), ProcessError>;
    pub async fn shutdown(&self) -> Result<ExitStatus, ProcessError>;
    pub async fn wait(&self) -> Result<ExitStatus, ProcessError>;
    pub fn id(&self) -> Option<u32>;
}

impl Drop for ManagedProcess {
    fn drop(&mut self); // Supervisor::request_shutdown — signals only, async work happens in the detached task
}
```

`spawn` builds the `tokio::process::Command` via
[spawn::build_command](/crates/clauders/agent/process/spawn.md), takes
ownership of stdin/stdout/stderr (erroring `ProcessError::Spawn` if any
stream was not captured), wraps stdout/stderr in
[StdoutLines/StderrBuffer](/crates/clauders/agent/process/pipes.md), and
starts a [Supervisor](/crates/clauders/agent/process/supervisor.md).
`shutdown` waits up to the configured grace period for natural exit, then
escalates to a forced (process-group, on Unix) kill and waits again.
`Drop` only *signals* teardown (synchronous); `kill_on_drop(true)` on the
spawn command is the final SIGKILL safety net if the async runtime is
already gone.

# Examples

```rust,no_run
use clauders::agent::process::ProcessConfig;
# async fn example() -> Result<(), clauders::agent::process::ProcessError> {
let cfg = ProcessConfig::new("/bin/echo");
let (process, io) = clauders::agent::process::ManagedProcess::spawn(&cfg)?;
let status = process.wait().await?;
# Ok(()) }
```

Related: [ProcessConfig](/crates/clauders/agent/process/spawn.md),
[ProcessIo](/crates/clauders/agent/process/io.md),
[Supervisor](/crates/clauders/agent/process/supervisor.md),
[ProcessError](/crates/clauders/agent/process/error.md),
[CliRuntime::connect](/crates/clauders/agent/cli/runtime.md) (owns the
`ManagedProcess` for the runtime's lifetime).

# Citations

1. `crates/clauders/src/agent/process/handle.rs`
