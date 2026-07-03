---
type: Rust Module
title: clauders::agent::process::supervisor::Supervisor
description: Supervisor — owns the spawned Child for its whole life, the single site that calls wait(); drives a graceful-shutdown-then-forced-kill-then-reap sequence in a detached task.
tags: [rust, sdk, agent, process, supervisor, shutdown]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/process/supervisor.rs
---

# Schema

```rust
type Outcome = Result<std::process::ExitStatus, ProcessError>;

pub(super) struct Supervisor {
    pid: Option<u32>,
    shutdown: Arc<Notify>,
    result_rx: watch::Receiver<Option<Outcome>>,
}

impl Supervisor {
    pub(super) fn spawn(child: Child, grace: Duration) -> Self;
    pub(super) const fn pid(&self) -> Option<u32>;
    pub(super) fn request_shutdown(self_: &Arc<Self>); // sync; fires Notify only
    pub(super) async fn shutdown(&self) -> Outcome;     // notify + await outcome
    pub(super) async fn wait(&self) -> Outcome;          // await outcome only
    async fn result(&self) -> Outcome;                   // watch-channel poll loop
}
```

`run(child, grace, shutdown)` (free function, the detached task body):
`tokio::select!` between the child's natural `wait()` and the `shutdown`
`Notify`. On a shutdown request: wait up to `grace` for natural exit; if
still running, `kill_tree` (Unix: `killpg` SIGKILL on the process group,
read live from the `Child` — never a stored pid, so a recycled pid can
never be signalled; other platforms: `child.start_kill()`); then a second
`grace` window for the reap, else `ProcessError::Timeout`.

Related: [ManagedProcess](/crates/clauders/agent/process/handle.md) (holds
an `Arc<Supervisor>`, `Drop` calls `request_shutdown`),
[ProcessConfig::shutdown_grace](/crates/clauders/agent/process/spawn.md),
[ProcessError](/crates/clauders/agent/process/error.md).

# Citations

1. `crates/clauders/src/agent/process/supervisor.rs`
