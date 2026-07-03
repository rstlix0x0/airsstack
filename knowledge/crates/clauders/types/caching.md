---
type: Rust Module
title: clauders::types::caching
description: Prompt-caching control values — CacheControl breakpoint marker and CacheTtl tier selector for the Messages API.
tags: [rust, sdk, newtype, caching, prompt-caching]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/caching.rs
---

Feature-gated behind `messages-caching`. Anthropic supports a single cache
tier family (`ephemeral`) with an optional TTL; `CacheControl` is
`#[non_exhaustive]` so future tiers are non-breaking additions.

# Schema

```rust
pub enum CacheTtl {
    FiveMinutes, // "5m", default when ttl omitted
    OneHour,     // "1h", 2x write price
}

#[non_exhaustive]
pub enum CacheControl {
    Ephemeral { ttl: Option<CacheTtl> }, // via CacheControl::ephemeral()
}
```

Attach to a [SystemSegment](/crates/clauders/types/system.md),
[TextBlock](/crates/clauders/messages/content.md),
[Tool](/crates/clauders/messages/tools.md), or a tool-result block to mark
a prompt-caching boundary.

# Examples

```rust
use clauders::types::CacheTtl;
let j = serde_json::to_string(&CacheTtl::OneHour).unwrap();
assert_eq!(j, r#""1h""#);
```

Related: [Usage cache-token fields](/crates/clauders/messages/response.md),
[TextBlock::with_cache](/crates/clauders/messages/content.md),
[Tool::cache_control](/crates/clauders/messages/tools.md).

# Citations

1. `crates/clauders/src/types/caching.rs`
