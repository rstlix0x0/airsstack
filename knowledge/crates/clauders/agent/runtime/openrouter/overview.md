---
type: Rust Module
title: clauders::agent::runtime::openrouter
description: Native OpenRouter chat-completions runtime module map — OpenRouterRuntime drives openrouter-rs's chat API in-process, the structural twin of the api (Messages API) adapter.
tags: [rust, sdk, agent, runtime, openrouter, native]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/openrouter/mod.rs
---

Structural twin of [api](/crates/clauders/agent/runtime/api/overview.md):
`convert` is the pure wire↔agent mapping seam, `tools` bridges the
in-process MCP registry to the OpenRouter function-tool surface, and
`runtime` owns [`OpenRouterRuntime`](/crates/clauders/agent/runtime/openrouter/runtime.md)
and the spawned agent loop. Unlike `api`, there is no `cache` submodule —
OpenRouter's chat-completions surface exposes no prompt-cache
`cache_control` equivalent. Part of the
[runtime layer](/crates/clauders/agent/runtime/overview.md).

# Schema

```rust
mod convert;
mod runtime;
mod tools;

pub use runtime::OpenRouterRuntime;
```

| Submodule | Concept |
| --- | --- |
| `runtime` | [OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md) — the `Runtime` impl and the spawned turn-loop |
| `convert` | [wire↔agent mapping](/crates/clauders/agent/runtime/openrouter/convert.md) |
| `tools` | [MCP↔OpenRouter function-tool bridge](/crates/clauders/agent/runtime/openrouter/tools.md) |

`Options.model` carries an OpenRouter model slug (e.g.
`"deepseek/deepseek-chat"`, `"anthropic/claude-sonnet-4-5"`) for this
runtime, not a bare Anthropic model name — the same `clauders::types::ModelId`
newtype is reused, but its string is interpreted as an OpenRouter id and
converted to `openrouter_rs::types::ModelId` at construction.

Related: [Runtime trait](/crates/clauders/agent/runtime.md),
[api overview](/crates/clauders/agent/runtime/api/overview.md) (structural
twin), [runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Citations

1. `crates/clauders/src/agent/runtime/openrouter/mod.rs`
