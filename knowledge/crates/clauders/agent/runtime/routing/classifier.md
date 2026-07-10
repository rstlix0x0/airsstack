---
type: Rust Module
title: clauders::agent::runtime::routing::classifier
description: Classifier — the routing decision seam (choose one catalog target for a prompt) — and RuntimeClassifier, its model-backed implementation that asks a driven Runtime to pick and parses the reply against the catalog.
tags: [rust, sdk, agent, runtime, routing, trait, classification]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/routing/classifier.rs
---

# Schema

```rust
#[async_trait]
pub trait Classifier: Send + Sync {
    async fn classify(&self, prompt: &Prompt, catalog: &[ModelCard]) -> Result<ModelId, RoutingError>;
}

pub struct RuntimeClassifier<R: Runtime> {
    runtime: R,
}

impl<R: Runtime> RuntimeClassifier<R> {
    pub const fn new(runtime: R) -> Self;
    fn selection_prompt(prompt: &Prompt, catalog: &[ModelCard]) -> String;
}
```

`Classifier::classify` must return one of the ids present in `catalog`;
[`RoutingRuntime::run`](/crates/clauders/agent/runtime/routing/runtime.md)
does not trust this — an out-of-catalog id or an `Err` both degrade to the
routing runtime's fallback rather than propagating.

`RuntimeClassifier<R>` wraps any [`Runtime`](/crates/clauders/agent/runtime.md)
— typically a cheap model — as the decision-maker:
`selection_prompt` renders a fixed instruction ("choose the single best
[candidate] .. reply with ONLY that model's id") followed by one
`"- <model>: <summary>"` line per [`ModelCard`](/crates/clauders/agent/runtime/routing/card.md)
and the task prompt itself. `classify` runs that rendered prompt through
`self.runtime.run(..)`, polls the returned
[`MessageStream`](/crates/clauders/agent/stream.md) directly via
`futures_core::poll_fn` (avoiding a `futures_util` production dependency)
to its terminal `Message::Result`, then finds the first catalog entry
whose `model.as_str()` is a substring of the reply. No result frame at all
→ `RoutingError::Classify("classifier produced no result")`; a reply
matching no candidate → `RoutingError::Parse { reply }`.

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::agent::RoutingError> {
use clauders::agent::{Classifier, ModelCard, RoutingSummary, RuntimeClassifier};
use clauders::agent::types::Prompt;
# fn judge_runtime() -> clauders::agent::MockRuntime { unimplemented!() }
let classifier = RuntimeClassifier::new(judge_runtime());
let catalog = vec![ModelCard {
    model: clauders::types::ModelId::custom("anthropic/claude-opus-4-7").expect("id"),
    summary: RoutingSummary::new("advanced")?,
}];
let _picked = classifier.classify(&Prompt::new("hard task"), &catalog).await?;
# Ok(())
# }
```

Related: [ModelCard / RoutingSummary](/crates/clauders/agent/runtime/routing/card.md),
[RoutingError](/crates/clauders/agent/runtime/routing/error.md),
[RoutingRuntime::run](/crates/clauders/agent/runtime/routing/runtime.md)
(the sole production caller of `classify`),
[Runtime trait](/crates/clauders/agent/runtime.md),
[MessageStream](/crates/clauders/agent/stream.md),
[MockRuntime](/crates/clauders/agent/mock.md) (used as the classifier's
judge runtime in tests).

# Citations

1. `crates/clauders/src/agent/runtime/routing/classifier.rs`
