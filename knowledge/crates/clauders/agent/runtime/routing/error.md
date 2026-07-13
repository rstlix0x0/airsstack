---
type: Rust Module
title: clauders::agent::runtime::routing::error::RoutingError
description: RoutingError — the closed-but-non_exhaustive set of failures raised while constructing or driving a RoutingRuntime, spanning summary validation, target/model-identity misconfiguration, and classification failures.
tags: [rust, sdk, agent, runtime, routing, error-handling]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/routing/error.rs
---

# Schema

```rust
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum RoutingError {
    #[error("routing summary must not be empty")]
    EmptySummary,
    #[error("routing summary too long: {got} chars exceeds max {max}")]
    SummaryTooLong { max: usize, got: usize },
    #[error("routing target has no model identity")]
    MissingModelId,
    #[error("duplicate routing target model: {0}")]
    DuplicateModel(ModelId),
    #[error("classification failed: {0}")]
    Classify(String),
    #[error("classifier reply matched no candidate: {reply}")]
    Parse { reply: String },
    #[error("no active routing target; call run() first")]
    NoActiveTarget,
}
```

Producers, by variant: `EmptySummary`/`SummaryTooLong` from
[`RoutingSummary::new`](/crates/clauders/agent/runtime/routing/card.md);
`MissingModelId`/`DuplicateModel` from
[`RoutingRuntimeBuilder::build`](/crates/clauders/agent/runtime/routing/builder.md)
(reading each target's [`Runtime::model()`](/crates/clauders/agent/runtime.md));
`Classify`/`Parse` from
[`RuntimeClassifier::classify`](/crates/clauders/agent/runtime/routing/classifier.md);
`NoActiveTarget` from
[`RoutingRuntime`](/crates/clauders/agent/runtime/routing/runtime.md)'s
control-op delegation when no `run()` has happened yet. `Classify` and
`Parse` are the only variants a live classification decision can raise —
[`RoutingRuntime::run`](/crates/clauders/agent/runtime/routing/runtime.md)
treats either (or an out-of-catalog id) as non-fatal and falls back rather
than propagating.

`#[non_exhaustive]` leaves room for future variants without a breaking
change; `Clone` lets a scripted `RoutingError` be reused across multiple
test assertions (see
[`Classifier` test doubles](/crates/clauders/agent/runtime/routing/classifier.md)).

# Examples

```rust
use clauders::agent::RoutingError;
assert!(RoutingError::EmptySummary.to_string().contains("empty"));
assert!(RoutingError::NoActiveTarget.to_string().contains("run()"));
```

Related: [RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md),
[RoutingRuntimeBuilder](/crates/clauders/agent/runtime/routing/builder.md),
[ModelCard / RoutingSummary](/crates/clauders/agent/runtime/routing/card.md),
[Classifier / RuntimeClassifier](/crates/clauders/agent/runtime/routing/classifier.md),
[AgentError](/crates/clauders/agent/error.md) (control-op delegation wraps
`RoutingError::NoActiveTarget.to_string()` inside `AgentError::Protocol`).

# Citations

1. `crates/clauders/src/agent/runtime/routing/error.rs`
