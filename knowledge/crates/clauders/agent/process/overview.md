---
type: Rust Module
title: clauders::agent::process
description: Protocol-blind subprocess management for arbitrary child processes — spawn, supervise, and tear down a child with graceful-then-forced shutdown, independent of the claude binary or the JSONL control protocol.
tags: [rust, sdk, agent, process, subprocess]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/process/mod.rs
---

Nothing in this module knows about the outer agent or its wire protocol —
that layer is built on top in [cli](/crates/clauders/agent/cli/overview.md).
Entry point: `ManagedProcess::spawn`.

# Schema

| Submodule | Concept |
| --- | --- |
| `error` | [ProcessError](/crates/clauders/agent/process/error.md) — all subprocess-layer failure modes |
| `handle` | [ManagedProcess](/crates/clauders/agent/process/handle.md) — owned handle, `Drop`-safe |
| `io` | [ProcessIo](/crates/clauders/agent/process/io.md) — the three pipe ends returned at spawn time |
| `pipes` | [StdoutLines / StderrBuffer](/crates/clauders/agent/process/pipes.md) |
| `spawn` | [ProcessConfig / build_command](/crates/clauders/agent/process/spawn.md) |
| `supervisor` | [Supervisor](/crates/clauders/agent/process/supervisor.md) — the detached task driving graceful→kill→reap |

Not responsible for interpreting the bytes on the pipes (the
[protocol](/crates/clauders/agent/protocol/overview.md) layer parses JSONL)
or retry/reconnect/session management (outside this module).

# Citations

1. `crates/clauders/src/agent/process/mod.rs`
