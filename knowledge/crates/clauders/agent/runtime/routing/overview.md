---
type: Rust Module
title: clauders::agent::runtime::routing
description: AI-driven per-request model routing module map — RoutingRuntime implements Runtime by classifying each prompt via a Classifier and delegating to the chosen backend runtime.
tags: [rust, sdk, agent, runtime, routing, module-map]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/routing/mod.rs
---

The meta-adapter of the [runtime layer](/crates/clauders/agent/runtime/overview.md):
wraps a set of backend [`Runtime`](/crates/clauders/agent/runtime.md)
implementors (typically an
[ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) or
[OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md)
per model) and, itself, implements `Runtime` — so a
[`Client`](/crates/clauders/agent/client.md) built over a `RoutingRuntime`
is indistinguishable from one built over any single backend.

# Schema

```rust
mod builder;
mod card;
mod classifier;
mod error;
mod runtime;

pub use builder::{NeedsFallback, Ready, RoutingRuntimeBuilder};
pub use card::{ModelCard, RoutingSummary};
pub use classifier::{Classifier, RuntimeClassifier};
pub use error::RoutingError;
pub use runtime::RoutingRuntime;
```

| Submodule | Concept |
| --- | --- |
| `runtime` | [RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md) — the `Runtime` impl that dispatches per `run()` |
| `builder` | [RoutingRuntimeBuilder (type-state)](/crates/clauders/agent/runtime/routing/builder.md) |
| `card` | [ModelCard / RoutingSummary](/crates/clauders/agent/runtime/routing/card.md) — the catalog shown to the classifier |
| `classifier` | [Classifier trait / RuntimeClassifier](/crates/clauders/agent/runtime/routing/classifier.md) — the decision seam |
| `error` | [RoutingError](/crates/clauders/agent/runtime/routing/error.md) |

Every target's catalog id is read from
[`Runtime::model()`](/crates/clauders/agent/runtime.md) — the default-`None`
method `ApiRuntime`/`OpenRouterRuntime` override with their construction-time
identity, and [`MockRuntime::with_model`](/crates/clauders/agent/mock.md)
sets in tests.

Related: [Runtime trait](/crates/clauders/agent/runtime.md),
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Citations

1. `crates/clauders/src/agent/runtime/routing/mod.rs`
