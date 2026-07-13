---
type: Rust Module
title: clauders::agent::runtime::routing::card
description: ModelCard and RoutingSummary — the routing catalog entry (a model identity paired with a short, validated, human-authored description) shown to the classifier when it chooses a target.
tags: [rust, sdk, agent, runtime, routing, newtype]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/routing/card.rs
---

# Schema

```rust
const MAX_SUMMARY_LEN: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutingSummary(String);

impl RoutingSummary {
    pub fn new(text: impl Into<String>) -> Result<Self, RoutingError>;
    pub fn as_str(&self) -> &str;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelCard {
    pub model: ModelId,        // sourced from Runtime::model()
    pub summary: RoutingSummary,
}
```

`RoutingSummary::new` trims surrounding whitespace, then validates:
`RoutingError::EmptySummary` if empty after trimming,
`RoutingError::SummaryTooLong { max: 512, got }` if the trimmed length
(in `char`s) exceeds `MAX_SUMMARY_LEN`. Once constructed, a
`RoutingSummary` is always non-empty and within bound.

`ModelCard` pairs one target's `Runtime::model()` identity with the
summary describing what it is best suited for — this is exactly what
[`RuntimeClassifier::selection_prompt`](/crates/clauders/agent/runtime/routing/classifier.md)
renders into its selection prompt (`"- <model>: <summary>"` per
candidate).

# Examples

```rust
use clauders::agent::{ModelCard, RoutingSummary};
use clauders::types::ModelId;

let card = ModelCard {
    model: ModelId::custom("deepseek/deepseek-chat").expect("id"),
    summary: RoutingSummary::new("cheap edits").expect("summary"),
};
assert_eq!(card.summary.as_str(), "cheap edits");
```

Related: [RoutingRuntimeBuilder](/crates/clauders/agent/runtime/routing/builder.md)
(assembles the `Vec<ModelCard>` catalog), [Classifier / RuntimeClassifier](/crates/clauders/agent/runtime/routing/classifier.md)
(reads the catalog to render its prompt), [RoutingError](/crates/clauders/agent/runtime/routing/error.md),
[Runtime::model()](/crates/clauders/agent/runtime.md) (the source of `model`).

# Citations

1. `crates/clauders/src/agent/runtime/routing/card.rs`
