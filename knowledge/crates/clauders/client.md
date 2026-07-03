---
type: Rust Module
title: clauders::client
description: Client<T> — the SDK handle every Messages/Models API call goes through, generic over the HTTP transport and cheap to clone via an internal Arc.
tags: [rust, sdk, client, transport]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/client.rs
---

`Client<T>` is the single handle every SDK call goes through. It is generic
over `T: HttpTransport` (static dispatch); cloning shares state through an
internal `Arc<ClientInner<T>>` rather than duplicating it.

Construct via [ClientBuilder](/crates/clauders/builder.md) — either
`Client::builder()` (feature-gated default `ReqwestTransport`, fallible
because TLS-backend init can fail) or `Client::builder_with_transport(t)`
(infallible, any custom transport).

# Schema

- `Client<T = DefaultTransportPlaceholder>` — `pub(crate) inner: Arc<ClientInner<T>>`.
- `DefaultClient` — type alias for `Client<ReqwestTransport>` (behind `transport-reqwest`).
- `ClientInner<T>` — `config: Config`, `transport: T`, `auth: Auth`, `retry: RetryPolicy`.
- `DefaultTransportPlaceholder` — resolves to `ReqwestTransport` when
  `transport-reqwest` is enabled; otherwise a stand-in whose `send` always
  errors, so `Client<T>`'s signature stays stable across feature configurations.

## Methods

- `config(&self) -> &Config`, `auth(&self) -> &Auth`, `retry(&self) -> &RetryPolicy` — narrow accessors.
- `ref_count(&self) -> usize` — live `Arc` strong-count, a best-effort diagnostic.
- `messages(&self) -> MessagesResource<'_, T>` (feature `messages`).
- `models(&self) -> ModelsResource<'_, T>` (feature `models`).
- `builder_with_transport(transport: T) -> ClientBuilder<Missing, T>` — infallible entry point.
- `Client::<ReqwestTransport>::builder() -> Result<ClientBuilder<Missing, ReqwestTransport>, BuildError>` (feature `transport-reqwest`).

`Debug` is implemented manually (`finish_non_exhaustive`) so credential
material is never printed.

# Examples

```rust,no_run
use clauders::prelude::*;
let client = Client::builder()?
    .api_key(ApiKey::new(std::env::var("ANTHROPIC_API_KEY").unwrap()).unwrap())
    .build()?;
let msg = client.messages().create(/* MessageRequest */ todo!()).await?;
# Ok::<(), clauders::Error>(())
```

Related: [ClientBuilder](/crates/clauders/builder.md),
[Config](/crates/clauders/config.md), [Auth](/crates/clauders/auth.md),
[messages module](/crates/clauders/messages/overview.md),
[models resource](/crates/clauders/models/resource.md).

# Citations

1. `crates/clauders/src/client.rs`
