---
type: Rust Module
title: clauders::agent::process::pipes
description: StdoutLines (line-oriented stdout reader) and StderrBuffer (bounded, continuously-drained stderr snapshot) — the two pipe-reading views over a spawned child.
tags: [rust, sdk, agent, process, io, pipes]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/process/pipes.rs
---

# Schema

```rust
const STDERR_CAP: usize = 64 * 1024; // upper bound on retained stderr bytes

pub struct StdoutLines(Lines<BufReader<ChildStdout>>);
impl StdoutLines {
    pub(crate) fn new(stdout: ChildStdout) -> Self;
    pub async fn next_line(&mut self) -> std::io::Result<Option<String>>;
}

pub struct StderrBuffer { inner: Arc<Mutex<Vec<u8>>> } // Clone
impl StderrBuffer {
    pub(crate) fn drain(mut stderr: ChildStderr) -> Self;
    pub fn snapshot(&self) -> String; // lossy UTF-8
}
```

`StderrBuffer::drain` spawns a background task that reads stderr to EOF so
the child can never block on a full stderr pipe; only the most recent
64 KiB are retained (`append_bounded` truncates from the front).

# Examples

```rust,no_run
# async fn example(mut stdout: clauders::agent::process::StdoutLines) {
while let Ok(Some(line)) = stdout.next_line().await {
    println!("{line}");
}
# }
```

Related: [ProcessIo](/crates/clauders/agent/process/io.md) (bundles both
types), [ManagedProcess::spawn](/crates/clauders/agent/process/handle.md)
(constructs both), [CliRuntime](/crates/clauders/agent/cli/runtime.md)
(`reader_loop` drives `StdoutLines::next_line` in a loop).

# Citations

1. `crates/clauders/src/agent/process/pipes.rs`
