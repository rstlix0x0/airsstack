---
type: Rust Module
title: clauders::agent::process::io::ProcessIo
description: ProcessIo — the three pipe ends (stdin, stdout, stderr) of a spawned child, bundled and handed to the caller at spawn time.
tags: [rust, sdk, agent, process, io]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/process/io.rs
---

Kept separate from [ManagedProcess](/crates/clauders/agent/process/handle.md)
because the two types have different ownership shapes: the caller owns
`ProcessIo` exclusively and can move it wherever it needs the I/O, while
`ManagedProcess` is the shutdown/wait control surface.

# Schema

```rust
pub struct ProcessIo {
    pub stdin: ChildStdin,
    pub stdout: StdoutLines,
    pub stderr: StderrBuffer,
}
```

Dropping `stdin` sends EOF to the child — the primary graceful-shutdown
signal for well-behaved children.

Related: [ManagedProcess::spawn](/crates/clauders/agent/process/handle.md)
(the sole producer), [StdoutLines / StderrBuffer](/crates/clauders/agent/process/pipes.md),
[CliRuntime::connect](/crates/clauders/agent/cli/runtime.md) (destructures
`ProcessIo` into stdin/stdout/stderr for the handshake and reader/writer tasks).

# Citations

1. `crates/clauders/src/agent/process/io.rs`
