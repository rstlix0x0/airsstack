---
type: Rust Module
title: clauders::config
description: Static, non-secret request configuration (base URL, API version, beta headers, timeout) carried by every Client.
tags: [rust, sdk, config]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/config.rs
---

`Config` holds the request metadata orthogonal to authentication
([Auth](/crates/clauders/auth.md) is separate) and retry policy
([RetryPolicy](/crates/clauders/retry.md) is separate). Fields are
crate-private; the only ways to set them are `Config::default()` and the
[ClientBuilder](/crates/clauders/builder.md).

# Schema

| Field | Type | Default |
| --- | --- | --- |
| `base_url` | `BaseUrl` | `https://api.anthropic.com/` |
| `anthropic_version` | `AnthropicVersion` | `AnthropicVersion::default()` (`2023-06-01`) |
| `anthropic_beta` | `Vec<BetaHeader>` | empty (header omitted) |
| `timeout` | `Duration` | 60 seconds |

Accessors: `base_url()`, `anthropic_version()`, `anthropic_beta()`, `timeout()`.

# Examples

```rust
use clauders::Config;
let c = Config::default();
assert_eq!(c.base_url().as_str(), "https://api.anthropic.com/");
```

Related: [ClientBuilder](/crates/clauders/builder.md) (the sole mutator),
[AnthropicVersion / BetaHeader](/crates/clauders/types/version.md),
[BaseUrl](/crates/clauders/types/base-url.md).

# Citations

1. `crates/clauders/src/config.rs`
