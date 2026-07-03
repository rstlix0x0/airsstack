---
type: Rust Module
title: clauders::agent::cli
description: The subprocess-backed Runtime adapter — locates and version-checks the claude binary, maps session options to its argv, runs the initialize handshake, and demultiplexes its output stream.
tags: [rust, sdk, agent, cli, subprocess]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/cli/mod.rs
---

Protocol-aware but defers all process lifecycle to the protocol-blind
[process module](/crates/clauders/agent/process/overview.md). All
submodules here are private (`mod`, not `pub mod`); only
[CliRuntime](/crates/clauders/agent/cli/runtime.md) is re-exported.

# Schema

| Submodule | Concept |
| --- | --- |
| `argv` | [build_argv / permission_mode_wire](/crates/clauders/agent/cli/argv.md) |
| `demux` | [Demux](/crates/clauders/agent/cli/demux.md) — routes inbound frames |
| `discovery` | [discover / check_version](/crates/clauders/agent/cli/discovery.md) |
| `dispatch` | [Dispatcher](/crates/clauders/agent/cli/dispatch.md) — answers inbound control requests |
| `handshake` | [initialize_request / parse_capabilities](/crates/clauders/agent/cli/handshake.md) |
| `runtime` | [CliRuntime](/crates/clauders/agent/cli/runtime.md) — the `Runtime` impl orchestrating all of the above |

# Citations

1. `crates/clauders/src/agent/cli/mod.rs`
