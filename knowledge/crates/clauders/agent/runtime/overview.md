---
type: Rust Module
title: clauders::agent::runtime
description: The runtime layer aggregator — declares the Runtime port and its adapters (api, cli, openrouter, routing, mock); everything above it (Client) is generic over the Runtime trait re-exported here.
tags: [rust, sdk, agent, runtime, module-map]
timestamp: 2026-07-11T00:00:00Z
resource: crates/clauders/src/agent/runtime/mod.rs
---

New aggregator module: `agent/runtime.rs` (a single file, documenting only
the `Runtime` trait) became the directory `agent/runtime/` when the CLI
adapter's siblings — native Messages-API and OpenRouter adapters, plus a
routing meta-adapter — were added. The trait itself lives at
[`agent/runtime/port.rs`](/crates/clauders/agent/runtime.md) (bundle
concept kept at its original path, `agent/runtime.md`, since its content —
the `Runtime` trait — is unchanged in kind).

# Schema

```rust
pub mod api;
pub mod cli;
pub mod openrouter;
pub(crate) mod permission_engine;
mod port;
pub mod routing;

pub use port::Runtime;

#[cfg(test)]
pub mod mock;
```

| Submodule | Concept |
| --- | --- |
| `port` | [Runtime trait](/crates/clauders/agent/runtime.md) — the single trait seam |
| `cli` | [CliRuntime overview](/crates/clauders/agent/cli/overview.md) — subprocess-backed adapter (default) |
| `api` | [ApiRuntime overview](/crates/clauders/agent/runtime/api/overview.md) — native Messages API adapter |
| `openrouter` | [OpenRouterRuntime overview](/crates/clauders/agent/runtime/openrouter/overview.md) — native OpenRouter chat-completions adapter |
| `permission_engine` (`pub(crate)`) | [RuleStore/evaluate](/crates/clauders/agent/runtime/permission_engine.md) — native permission enforcement consulted by `api` (not `cli` or `openrouter`) |
| `routing` | [RoutingRuntime overview](/crates/clauders/agent/runtime/routing/overview.md) — meta-adapter dispatching per-turn to one of the others |
| `mock` (`cfg(test)`) | [MockRuntime](/crates/clauders/agent/mock.md) — subprocess-free test double |

`agent::mod.rs` re-exports selectively from here: `Runtime`,
`runtime::api::{ApiRuntime, CachePolicy}`, `runtime::cli::CliRuntime`,
`runtime::openrouter::OpenRouterRuntime`, and
`runtime::routing::{Classifier, ModelCard, RoutingError, RoutingRuntime,
RoutingRuntimeBuilder, RoutingSummary, RuntimeClassifier}` — so
`clauders::agent::{ApiRuntime, CliRuntime, OpenRouterRuntime,
RoutingRuntime, ..}` are the public paths even though the adapters live
under this nested `runtime::` tree.

The `api` and `openrouter` adapters are structural twins: each pairs a
`runtime` submodule (owns the struct and the spawned agent loop) with a
pure `convert` mapping module and a `tools` module bridging the in-process
[`SdkMcpRegistry`](/crates/clauders/agent/cli/dispatch.md) to that
backend's tool-calling surface; `api` additionally has a `cache` module
for prompt-cache breakpoint placement, since only the Messages API
exposes `cache_control`.

# Citations

1. `crates/clauders/src/agent/runtime/mod.rs`
