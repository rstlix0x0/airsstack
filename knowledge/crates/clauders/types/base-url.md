---
type: Rust Newtype
title: clauders::types::BaseUrl
description: Validated base-URL newtype restricted to http/https schemes, keeping the raw url::Url type off the public SDK surface.
tags: [rust, sdk, newtype, url]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/types/base_url.rs
---

Exists so a `url` crate version bump can never become a breaking change for
downstream callers — the inner `url::Url` stays private; only a string view
is exposed.

# Schema

```rust
pub struct BaseUrl(url::Url);

pub enum InvalidBaseUrl {
    Malformed(String),          // not a valid absolute URL
    UnsupportedScheme(String),  // scheme other than http/https
}
```

`BaseUrl::parse(s: &str) -> Result<Self, InvalidBaseUrl>` accepts `http`
(for local proxies/test servers) and `https` only; rejects `file`, `data`,
`ftp`, etc. Request-URI path assembly (joining an endpoint path onto the
validated base) is the request layer's job, not this type's.

# Examples

```rust
use clauders::types::BaseUrl;
let base = BaseUrl::parse("https://api.anthropic.com").expect("valid https URL");
assert_eq!(base.as_str(), "https://api.anthropic.com/");
```

Related: [Config::base_url](/crates/clauders/config.md),
[ClientBuilder::base_url](/crates/clauders/builder.md).

# Citations

1. `crates/clauders/src/types/base_url.rs`
