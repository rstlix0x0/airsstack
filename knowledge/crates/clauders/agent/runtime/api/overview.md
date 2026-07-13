---
type: Rust Module
title: clauders::agent::runtime::api
description: Native Messages API runtime module map — ApiRuntime drives POST /v1/messages in-process, reimplementing the send/stream/run-tools/loop cycle the CLI protocol otherwise handles.
tags: [rust, sdk, agent, runtime, messages-api, native]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/api/mod.rs
---

[`ApiRuntime`](/crates/clauders/agent/runtime/api/runtime.md) implements
the [`Runtime`](/crates/clauders/agent/runtime.md) seam by driving the
agent loop against `POST /v1/messages` directly — no `claude` binary, no
control protocol — emitting the same
[`Message`](/crates/clauders/agent/message.md) frames the core consumes.
Part of the [runtime layer](/crates/clauders/agent/runtime/overview.md);
structural twin of [openrouter](/crates/clauders/agent/runtime/openrouter/overview.md).

# Schema

```rust
mod cache;
mod convert;
mod runtime;
mod tools;

pub use cache::CachePolicy;
pub use runtime::ApiRuntime;
```

| Submodule | Concept |
| --- | --- |
| `runtime` | [ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) — the `Runtime` impl and the spawned turn-loop |
| `convert` | [wire↔agent mapping](/crates/clauders/agent/runtime/api/convert.md) — pure, the unit-test seam |
| `cache` | [CachePolicy](/crates/clauders/agent/runtime/api/cache.md) — prompt-cache breakpoint placement |
| `tools` | [MCP↔Messages-API tool bridge](/crates/clauders/agent/runtime/api/tools.md) |

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::agent::AgentError> {
use clauders::agent::Client as AgentClient;
use clauders::agent::{ApiRuntime, CachePolicy, Options};
use clauders::Client as WireClient;
use clauders::types::{ApiKey, MaxTokens, ModelId};

let wire = WireClient::builder()
    .expect("transport")
    .api_key(ApiKey::new("sk-ant-…").expect("key"))
    .build()
    .expect("client");
let options = Options::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(1024).expect("non-zero"))
    .build();
let runtime = ApiRuntime::new(wire, options)?.with_cache_policy(CachePolicy::Prefix);
let agent = AgentClient::with_runtime(runtime);
let _stream = agent.query("Hello").await?;
# Ok(())
# }
```

Related: [Runtime trait](/crates/clauders/agent/runtime.md),
[runtime layer overview](/crates/clauders/agent/runtime/overview.md),
[OpenRouterRuntime overview](/crates/clauders/agent/runtime/openrouter/overview.md)
(structural twin), [Options](/crates/clauders/agent/options.md),
[messages/request](/crates/clauders/messages/request.md) (the wire
`MessageRequest` this runtime builds).

# Citations

1. `crates/clauders/src/agent/runtime/api/mod.rs`
