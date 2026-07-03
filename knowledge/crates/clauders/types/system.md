---
type: Rust Module
title: clauders::types::system
description: System-prompt request types — SystemPrompt (bare string or typed segment array) and SystemSegment, the addressable-chunk system prompt shape.
tags: [rust, sdk, system-prompt]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/system.rs
---

Two wire-format shapes are accepted, chosen via an untagged enum recovered
purely from JSON shape.

# Schema

```rust
#[serde(untagged)]
pub enum SystemPrompt {
    Text(String),               // serializes as a bare JSON string
    Segments(Vec<SystemSegment>), // serializes as a JSON array
}

pub struct SystemSegment { /* kind: SystemSegmentKind, text, cache_control?, ... */ }
pub enum SystemSegmentKind { /* e.g. Text */ }
```

Use `SystemPrompt::Text` for the common single-string case (smallest wire
payload). Use `SystemPrompt::Segments` when the prompt is composed of
independently addressable chunks — e.g. stable, cache-friendly chunks mixed
with per-call-varying ones.

# Examples

```rust
use clauders::types::SystemPrompt;
let p = SystemPrompt::text("You are terse.");
assert_eq!(serde_json::to_string(&p).unwrap(), "\"You are terse.\"");
```

Related: [MessageRequestBuilder::system](/crates/clauders/messages/request.md),
[CacheControl](/crates/clauders/types/caching.md) (attachable to a `SystemSegment`).

# Citations

1. `crates/clauders/src/types/system.rs`
