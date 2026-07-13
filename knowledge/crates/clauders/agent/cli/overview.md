---
type: Rust Module
title: clauders::agent::runtime::cli
description: The subprocess-backed Runtime adapter — locates and version-checks the claude binary, maps session options to its argv, runs the initialize handshake, and demultiplexes its output stream. Now nested under agent::runtime alongside the api/openrouter/routing adapters.
tags: [rust, sdk, agent, cli, subprocess]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/cli/mod.rs
---

Relocated from `agent/cli/` to `agent/runtime/cli/` when the runtime layer
regrouped its adapters — see the
[runtime layer overview](/crates/clauders/agent/runtime/overview.md) for
the full `api`/`cli`/`openrouter`/`routing`/`port` picture. Module doc
comment and submodule set are unchanged by the move.

Protocol-aware but defers all process lifecycle to the protocol-blind
[process module](/crates/clauders/agent/process/overview.md). All
submodules here are private (`mod`, not `pub mod`); only
[CliRuntime](/crates/clauders/agent/cli/runtime.md) is re-exported (from
`agent::runtime`, then re-exported again at `agent::CliRuntime`).

# Schema

| Submodule | Concept |
| --- | --- |
| `argv` | [build_argv / permission_mode_wire](/crates/clauders/agent/cli/argv.md) |
| `demux` | [Demux](/crates/clauders/agent/cli/demux.md) — routes inbound frames |
| `discovery` | [discover / check_version](/crates/clauders/agent/cli/discovery.md) |
| `dispatch` | [Dispatcher](/crates/clauders/agent/cli/dispatch.md) — answers inbound control requests (including `mcp_message`) |
| `handshake` | [initialize_request / parse_capabilities](/crates/clauders/agent/cli/handshake.md) |
| `runtime` | [CliRuntime](/crates/clauders/agent/cli/runtime.md) — the `Runtime` impl orchestrating all of the above |

Sibling adapters under the same `agent::runtime` parent:
[api](/crates/clauders/agent/runtime/api/overview.md) (native Messages API),
[openrouter](/crates/clauders/agent/runtime/openrouter/overview.md) (native
OpenRouter chat-completions), [routing](/crates/clauders/agent/runtime/routing/overview.md)
(meta-adapter dispatching per-turn to one of the others),
[port](/crates/clauders/agent/runtime.md) (the `Runtime` trait itself).

# Citations

1. `crates/clauders/src/agent/runtime/cli/mod.rs`
