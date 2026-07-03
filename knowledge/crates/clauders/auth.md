---
type: Rust Module
title: clauders::auth
description: Auth — the closed set of authentication schemes attached to every outgoing Messages/Models API request.
tags: [rust, sdk, auth]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/auth.rs
---

`Auth` lives in its own module so the set of accepted auth shapes evolves
independently of [Config](/crates/clauders/config.md) (static request
metadata) and the transport boundary (auth-agnostic).

# Schema

```rust
pub enum Auth {
    ApiKey(ApiKey),
}
```

- `Auth::api_key(&self) -> Option<&ApiKey>` — narrow accessor for the common
  path; returns `None` for any future non-API-key variant. Pattern matches
  against `Auth` should use a `_` arm so future additions (e.g. a
  `Bearer`-style variant) are non-breaking for callers.

Related: [ApiKey newtype](/crates/clauders/types/api-key.md),
[Client::auth()](/crates/clauders/client.md).

# Citations

1. `crates/clauders/src/auth.rs`
