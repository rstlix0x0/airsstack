---
type: Rust Newtype
title: clauders::types::version
description: Anthropic API version and beta-header newtypes — AnthropicVersion (anthropic-version header) and BetaHeader (anthropic-beta header values).
tags: [rust, sdk, newtype, http-headers, versioning]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/version.rs
---

# Schema

```rust
pub struct AnthropicVersion(VersionRepr); // Static(&'static str) | Owned(String)

pub enum InvalidAnthropicVersion { Empty, BadChars }

pub struct BetaHeader(/* similar validated newtype */);
pub enum InvalidBetaHeader { /* ... */ }
```

`AnthropicVersion::V_2023_06_01` is the current stable version constant
(also the `Default`). `AnthropicVersion::custom(s)` supports forward-compat
with Anthropic releases this SDK version predates.

# Examples

```rust
use clauders::types::AnthropicVersion;
assert_eq!(AnthropicVersion::V_2023_06_01.as_str(), "2023-06-01");
assert_eq!(AnthropicVersion::default(), AnthropicVersion::V_2023_06_01);
```

Related: [Config::anthropic_version/anthropic_beta](/crates/clauders/config.md),
[ClientBuilder::anthropic_version/set_anthropic_beta](/crates/clauders/builder.md).

# Citations

1. `crates/clauders/src/types/version.rs`
