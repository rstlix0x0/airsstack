---
type: Rust Module
title: clauders::agent::runtime::routing::builder::RoutingRuntimeBuilder
description: RoutingRuntimeBuilder<C, State> — the type-state builder for RoutingRuntime; a classifier is required up front, a fallback target must be set before build() is even callable, and build() validates every target exposes a distinct model identity.
tags: [rust, sdk, agent, runtime, routing, builder, type-state]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/routing/builder.rs
---

A compile-time-enforced two-phase builder: `target()` is available in
every state, but `build()` exists only once a fallback has been set — the
uninitialized-fallback mistake is a compile error, not a runtime one.

# Schema

```rust
pub struct NeedsFallback; // initial state
pub struct Ready;         // reached via fallback_target(); build() available

pub struct RoutingRuntimeBuilder<C, State> {
    classifier: C,
    targets: Vec<(Arc<dyn Runtime>, RoutingSummary)>,
    fallback: Option<Arc<dyn Runtime>>,
    _state: PhantomData<State>,
}

impl<C: Classifier> RoutingRuntime<C> {
    pub fn builder(classifier: C) -> RoutingRuntimeBuilder<C, NeedsFallback>;
}

impl<C, State> RoutingRuntimeBuilder<C, State> {
    pub fn target(mut self, runtime: impl Runtime + 'static, summary: RoutingSummary) -> Self;
}

impl<C> RoutingRuntimeBuilder<C, NeedsFallback> {
    pub fn fallback_target(self, runtime: impl Runtime + 'static, summary: RoutingSummary) -> RoutingRuntimeBuilder<C, Ready>;
}

impl<C: Classifier> RoutingRuntimeBuilder<C, Ready> {
    pub fn build(self) -> Result<RoutingRuntime<C>, RoutingError>;
}
```

`target()` boxes the runtime as `Arc<dyn Runtime>` and stashes it with its
`RoutingSummary`; available regardless of state — optional targets can be
registered before or interleaved with setting the fallback.
`fallback_target()` does the same but ALSO records the fallback `Arc`
separately and transitions `NeedsFallback` → `Ready`, unlocking `build()`.

`build()` reads each target's id via
[`Runtime::model()`](/crates/clauders/agent/runtime.md):

- `RoutingError::MissingModelId` if any target (including the fallback)
  returns `None` — a routing target must have a fixed identity.
- `RoutingError::DuplicateModel(id)` if two targets resolve to the same
  `ModelId`.

On success, assembles the `targets` map, the `catalog: Vec<ModelCard>`
(one entry per target, pairing its id with its `RoutingSummary`), and the
fallback id, then calls
[`RoutingRuntime::from_parts`](/crates/clauders/agent/runtime/routing/runtime.md).

# Examples

```rust,no_run
# fn example() -> Result<(), clauders::agent::RoutingError> {
use clauders::agent::{ModelCard, RoutingRuntime, RoutingSummary, Classifier};
# struct MyClassifier;
# #[async_trait::async_trait]
# impl Classifier for MyClassifier {
#     async fn classify(&self, _p: &clauders::agent::types::Prompt, _c: &[ModelCard]) -> Result<clauders::types::ModelId, clauders::agent::RoutingError> { unreachable!() }
# }
// RoutingRuntime::builder(MyClassifier)
//     .target(cheap_runtime, RoutingSummary::new("cheap edits")?)
//     .fallback_target(advanced_runtime, RoutingSummary::new("hard reasoning")?)
//     .build()?;
# Ok(())
# }
```

Related: [RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md)
(the assembled product), [ModelCard / RoutingSummary](/crates/clauders/agent/runtime/routing/card.md),
[Classifier](/crates/clauders/agent/runtime/routing/classifier.md),
[RoutingError](/crates/clauders/agent/runtime/routing/error.md),
[Runtime trait (`model()`)](/crates/clauders/agent/runtime.md).

# Citations

1. `crates/clauders/src/agent/runtime/routing/builder.rs`
