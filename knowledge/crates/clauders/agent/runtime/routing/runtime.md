---
type: Rust Module
title: clauders::agent::runtime::routing::runtime::RoutingRuntime
description: RoutingRuntime<C> — a Runtime that routes each run() to one of several backend runtimes via a Classifier, falling back on a classification failure or an unknown pick, and delegating control ops to the last-selected target.
tags: [rust, sdk, agent, runtime, routing, capability-intersection]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/routing/runtime.rs
---

# Schema

```rust
pub struct RoutingRuntime<C: Classifier> {
    classifier: C,
    targets: HashMap<ModelId, Arc<dyn Runtime>>,
    catalog: Vec<ModelCard>,
    fallback: ModelId,
    active: Arc<Mutex<Option<ModelId>>>,   // last-selected target, for control-op delegation
    capabilities: Capabilities,             // intersection across all targets
}

impl<C: Classifier> RoutingRuntime<C> {
    pub(super) fn from_parts(
        classifier: C,
        targets: HashMap<ModelId, Arc<dyn Runtime>>,
        catalog: Vec<ModelCard>,
        fallback: ModelId,
    ) -> Self;
}
```

Prefer [`RoutingRuntime::builder`](/crates/clauders/agent/runtime/routing/builder.md)
over `from_parts` directly — the builder validates that `fallback` is
present in `targets` and every catalog entry has a matching target;
`from_parts` assumes it.

# `run()` — classify, validate, delegate

Classifies the prompt via `self.classifier.classify(&prompt, &self.catalog)`.
Any classifier error, OR a returned id not present in `targets`, degrades
to `self.fallback` rather than failing the turn — routing decisions are
never fatal. The resolved id is recorded as `active` (read by later control
ops), then the run is delegated whole to that target's
`Runtime::run(prompt)`.

# Control-op delegation

`interrupt`/`set_model`/`set_permission_mode`/`mcp_status` all go through
`active_delegate()`, which reads `active` and looks it up in `targets`. If
no `run()` has happened yet, `active` is `None` and every control op
returns `AgentError::Protocol { detail: RoutingError::NoActiveTarget.to_string() }` —
there is nothing to delegate to before the first turn selects a target.

# Capability intersection

`capabilities()` returns the *conservative floor* across every target:
`intersect_capabilities` folds each target's `supported_hook_events` and
`supported_control_methods` via set intersection, under the fixed marker
`protocol_version: "routing-1.0"`. A capability is reported as supported by
the routing runtime only if every possible backend honors it — since which
backend actually serves a given turn is not known ahead of the classifier's
decision.

# Examples

```rust,no_run
# fn example() {
// See RoutingRuntimeBuilder for the constructor path; RoutingRuntime is
// assembled from a Classifier plus at least one fallback target.
# }
```

Related: [Runtime trait](/crates/clauders/agent/runtime.md) (`model()` is
the source of each target's catalog key),
[RoutingRuntimeBuilder](/crates/clauders/agent/runtime/routing/builder.md)
(the validated constructor), [ModelCard / RoutingSummary](/crates/clauders/agent/runtime/routing/card.md),
[Classifier / RuntimeClassifier](/crates/clauders/agent/runtime/routing/classifier.md),
[RoutingError](/crates/clauders/agent/runtime/routing/error.md),
[MockRuntime](/crates/clauders/agent/mock.md) (its own test suite's
targets, built via `with_model`/`with_capabilities`),
[ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) and
[OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md)
(typical real targets — e.g. a cheap OpenRouter model plus an advanced
Claude model).

# Citations

1. `crates/clauders/src/agent/runtime/routing/runtime.rs`
