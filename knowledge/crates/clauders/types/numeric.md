---
type: Rust Newtype
title: clauders::types::numeric
description: Bounded numeric newtypes used in MessageRequest sampling parameters — MaxTokens, Temperature, TopK, TopP.
tags: [rust, sdk, newtype, sampling-parameters]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/numeric.rs
---

Each newtype validates its range at construction so downstream code trusts
the type as proof, without re-checking bounds before every request.

# Schema

| Type | Inner | Valid range | Error |
| --- | --- | --- | --- |
| `MaxTokens` | `u32` | non-zero (no SDK-side upper bound — server enforces per-model caps) | `InvalidMaxTokens` |
| `Temperature` | `f32` | `0.0..=1.0` | `InvalidTemperature { value }` |
| `TopK` | — | (analogous bounded newtype) | `InvalidTopK` |
| `TopP` | — | (analogous bounded newtype) | `InvalidTopP` |

`MaxTokens::new(n: u32) -> Result<Self, InvalidMaxTokens>`,
`Temperature::new(f32) -> Result<Self, InvalidTemperature>`, and the
`TopK`/`TopP` equivalents; each exposes `get()`.

# Examples

```rust
use clauders::types::{MaxTokens, Temperature};
assert_eq!(MaxTokens::new(1024).expect("non-zero").get(), 1024);
assert!(MaxTokens::new(0).is_err());
assert!(Temperature::new(0.7).is_ok());
assert!(Temperature::new(1.5).is_err());
```

Related: [MessageRequestBuilder::max_tokens/temperature/top_p/top_k](/crates/clauders/messages/request.md).

# Citations

1. `crates/clauders/src/types/numeric.rs`
