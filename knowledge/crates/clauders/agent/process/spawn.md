---
type: Rust Module
title: clauders::agent::process::spawn
description: ProcessConfig — declarative spawn parameters (program, args, cwd, env, shutdown grace) and build_command, which assembles a tokio::process::Command with piped stdio and a Unix process-group leader.
tags: [rust, sdk, agent, process, spawn]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/process/spawn.rs
---

# Schema

```rust
pub struct ProcessConfig {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub shutdown_grace: Duration, // ProcessConfig::new() default: 5s
}

pub(super) fn build_command(cfg: &ProcessConfig) -> Command;
```

`build_command` pipes all three stdio streams, sets `kill_on_drop(true)` as
a last-resort safety net, and on Unix makes the child the leader of a new
process group (`process_group(0)`, so `pgid == child pid`) so the whole
group — including the child's own descendants — can be signalled at once.

# Examples

```rust
use clauders::agent::process::ProcessConfig;
let cfg = ProcessConfig::new("/bin/echo");
assert!(cfg.args.is_empty());
assert_eq!(cfg.shutdown_grace, std::time::Duration::from_secs(5));
```

Related: [ManagedProcess::spawn](/crates/clauders/agent/process/handle.md)
(calls `build_command`), [Supervisor::run](/crates/clauders/agent/process/supervisor.md)
(uses `shutdown_grace` for both the graceful-exit and post-kill windows),
[cli::argv::build_argv](/crates/clauders/agent/cli/argv.md) (produces the
`args` this config carries).

# Citations

1. `crates/clauders/src/agent/process/spawn.rs`
