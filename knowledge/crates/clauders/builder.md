---
type: Rust Module
title: clauders::builder
description: Type-state builder for Client<T> that makes api_key-before-build a compile-time requirement rather than a runtime error.
tags: [rust, sdk, builder, type-state]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/builder.rs
---

`ClientBuilder<Key, T>` encodes "`api_key` must be set before `build()`" in
the type system: `build()` only exists once the first type parameter is the
`Present` marker. There is no runtime `BuilderError::MissingApiKey` — a
`ClientBuilder<Missing, _>` simply does not have a `build` method.

# Schema

- `BuilderApiKeyState` — sealed trait; the closed inhabitant set is `Missing` and `Present`.
- `ClientBuilder<Key, T>` — `fields: ClientBuilderFields`, `transport: T`, `_key: PhantomData<Key>`.
- `ClientBuilderFields` (private) — `api_key`, `version` (default `AnthropicVersion::V_2023_06_01`), `beta: Vec<BetaHeader>`, `timeout`, `retry`, `base_url`.

## Methods

- `ClientBuilder::<Missing, T>::api_key(self, key: ApiKey) -> ClientBuilder<Present, T>` — the state transition.
- Available regardless of state: `anthropic_version`, `set_anthropic_beta`,
  `add_anthropic_beta`, `timeout`, `retry`, `base_url`.
- `ClientBuilder::<Present, T>::build(self) -> Result<Client<T>, BuildError>` —
  only callable once `Present`; the internal `expect` is an unreachable
  safety net guaranteed by the type-state.

Optional fields set *before* the `api_key` transition survive it — the
whole `ClientBuilderFields` struct moves as one value on transition, so no
per-field copy can be forgotten.

# Examples

```rust,no_run
use clauders::prelude::*;
let client = Client::builder()?
    .anthropic_version(AnthropicVersion::V_2023_06_01)
    .api_key(ApiKey::new("sk-ant-...").unwrap())
    .build()?;
# Ok::<(), clauders::Error>(())
```

Related: [Client](/crates/clauders/client.md), [Config](/crates/clauders/config.md),
[RetryPolicy](/crates/clauders/retry.md),
[messages request builder](/crates/clauders/messages/request.md) (same
type-state pattern, independently scoped).

# Citations

1. `crates/clauders/src/builder.rs`
