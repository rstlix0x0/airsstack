---
type: Rust Newtype
title: clauders::types::ApiKey
description: Secret-protected newtype wrapping the Anthropic API key so it never appears in Debug output; validated non-empty ASCII-printable at construction.
tags: [rust, sdk, newtype, secrets, auth]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/api_key.rs
---

Wraps a `secrecy::SecretString` so `Debug` prints `ApiKey("***")` rather
than the raw key. Use `expose_secret()` to obtain the value for the
`x-api-key` header.

# Schema

```rust
pub struct ApiKey(SecretString);

pub enum InvalidApiKey {
    Empty,       // "" rejected
    NonPrintable, // non-ASCII or whitespace bytes rejected
}
```

`ApiKey::new(raw: impl Into<String>) -> Result<Self, InvalidApiKey>` is the
only constructor.

# Examples

```rust
use clauders::types::ApiKey;
use secrecy::ExposeSecret;
let key = ApiKey::new("sk-test-abcdef").expect("valid key");
assert_eq!(key.expose_secret(), "sk-test-abcdef");
let dbg = format!("{key:?}");
assert!(!dbg.contains("sk-test"));
```

Related: [Auth::ApiKey](/crates/clauders/auth.md),
[ClientBuilder::api_key](/crates/clauders/builder.md).

# Citations

1. `crates/clauders/src/types/api_key.rs`
