# Spec: Extract `airs-transport` — a layered transport substrate

- **Date:** 2026-06-06
- **Status:** Proposed (awaiting user approval)
- **Scope:** One objective — create `crates/airs-transport` with a layered transport abstraction and migrate `clauders` and `openrouter-rs` onto it.

## 1. Motivation

`crates/clauders` and `crates/openrouter-rs` were built from the same SDK skeleton, so a
band of low-level HTTP plumbing is duplicated between them. The duplicated code carries
**zero domain knowledge** — it does not name a provider, an endpoint, an API key format,
or a wire error envelope. Two costs follow:

1. **Maintenance drift.** The copies have already diverged: `openrouter-rs` refactored the
   `reqwest` error-classification logic into a pure, unit-tested function while `clauders`
   still inlines it.
2. **No single source of truth** for the transport contract every future SDK in this
   workspace needs.

Extracting the generic substrate into one crate removes the duplication and collapses the
drift to a single canonical implementation. This spec additionally introduces a **layered
transport abstraction** so the substrate is not hard-bound to HTTP: a generic `Transport`
contract, an HTTP specialization on top of it, and `reqwest` as the concrete HTTP
implementer.

## 2. The boundary test (what is "generic")

A single rule decides whether code belongs in `airs-transport` or stays in a consumer crate:

> **Does the code name a provider, an endpoint, an API-key format, a model catalog, a
> sampling range, or a wire error envelope?**
> **Yes → it stays in the consumer crate. No → it is eligible for `airs-transport`.**

This test names the crate: `airs-transport` (not `utils`, `libs`, `core`, or `airs-http`),
chosen so contributors read "transport" and do not dump domain code here, and so the name
admits non-HTTP transports without a rename.

## 3. Transport abstraction layering (the central design)

Three layers, from most generic to most concrete:

```
Transport        — abstraction: send a request, get a response. Knows nothing about HTTP.
   ▲ is-a
HttpTransport    — abstraction: a Transport whose request/response ARE the HTTP types.
   ▲ implements
ReqwestTransport — concrete: implements the transport via reqwest.
```

### 3.1 `Transport` — the generic contract

```rust
#[async_trait::async_trait]
pub trait Transport: Send + Sync + 'static {
    type Request: Send;
    type Response: Send;
    type Error: Send;
    async fn send(&self, req: Self::Request) -> Result<Self::Response, Self::Error>;
}
```

`Transport` names no HTTP concept. A future non-HTTP transport (a message queue, an
in-memory test double, a gRPC channel) implements `Transport` with its own associated
types. The single operation is `send`.

**Method name is `send`, not `call`.** Naming it `call` (the Tower verb) would rename the
`mockall`-generated `expect_send()` to `expect_call()` and force edits across every
mock-based test and every resource call site in both consumer crates. `send` keeps the
generic meaning ("send a request, get a response") while preserving the entire existing
test and call surface.

### 3.2 `HttpTransport` — the HTTP specialization (is-a Transport)

```rust
pub trait HttpTransport:
    Transport<
        Request = http::Request<Bytes>,
        Response = http::Response<BodyStream>,
        Error = TransportError,
    >
{
}

impl<T> HttpTransport for T where
    T: Transport<
        Request = http::Request<Bytes>,
        Response = http::Response<BodyStream>,
        Error = TransportError,
    >
{
}
```

`HttpTransport` is a **marker sub-trait with a blanket impl**: any `Transport` whose
associated types are the HTTP types *is* an `HttpTransport`, automatically. Implementers
write `impl Transport for X` once and get `HttpTransport` for free. The blanket impl is sound
because `HttpTransport` is a local trait. Consumers keep bounding their generic client on
`T: HttpTransport`; calling `t.send(req)` resolves through the `Transport` supertrait, so any
module performing a transport send imports `Transport` alongside `HttpTransport`.

### 3.3 `ReqwestTransport` — the concrete HTTP implementer

```rust
impl Transport for ReqwestTransport {
    type Request = http::Request<Bytes>;
    type Response = http::Response<BodyStream>;
    type Error = TransportError;
    async fn send(&self, req: http::Request<Bytes>)
        -> Result<http::Response<BodyStream>, TransportError> { /* … */ }
}
```

### 3.4 Why not "HTTP composes a lower byte-Transport"

The layering is **is-a** (HTTP is a kind of Transport), not **stacked composition** (HTTP
framing built on top of a separately-swappable byte channel). `reqwest` is monolithic —
connection, TLS, HTTP framing, and byte transport are fused — so it cannot sit *on top of* a
lower byte transport. `ReqwestTransport` therefore implements the HTTP layer directly. A
stacked split would require owning HTTP framing over raw bytes (hyper or lower), which is
explicitly out of scope.

### 3.5 Why not Tower `Service`

`tower::Service` is the canonical generic version of this contract and brings the `Layer`
middleware ecosystem. It was considered and rejected for now: it pulls `tower` into a
deliberately dependency-slim crate, its `poll_ready`/`call`/`Future`-assoc-type ergonomics
are heavier than a one-method `async_trait`, and `reqwest` is not a `Service` natively. The
hand-rolled `Transport` trait is `Service`-shaped enough that a later migration to Tower is
possible without redesigning consumers. No middleware need exists today (clauders' `retry`
module covers the one case).

## 4. Scope

### In scope (moves into / is created in `airs-transport`)

| Item | Source / nature |
|------|-----------------|
| `Transport` generic trait (`Request`/`Response`/`Error` assoc types, `async fn send`) | **new** (this spec) |
| `HttpTransport` marker sub-trait + blanket impl | derived from the existing `HttpTransport` |
| `BodyStream` type alias | `src/transport/body.rs` |
| `ReqwestTransport` + `reqwest`→`TransportError` classifier + TLS detection + `BodyStreamAdapter` | `src/transport/reqwest_impl.rs` |
| `MockHttpTransport` (`mockall`, now mocking `Transport`) | `src/transport/mock.rs` |
| `TransportError` enum + `is_retryable` | `src/error.rs` |
| `collect_body` + `MAX_RESPONSE_BODY_BYTES` | `src/wire_helpers.rs` |

### Out of scope (stays in each consumer crate)

- Domain newtypes: `ApiKey`, `BaseUrl`, `ModelId`, the `numeric` sampling types.
- `Config`, `headers` constants, `Auth`.
- `ClientBuilder` / `Client` (generic over `<T: HttpTransport>`, bound to each crate's `Config`).
- Error envelopes: each crate's top-level `Error`, `ApiError`/`ApiErrorBody`, `BuildError`,
  and `decode_api_error_from_parts` / `parse_retry_after` in `wire_helpers.rs`.
- SSE streaming (`messages/streaming.rs`, `chat/stream.rs`).
- All API-surface types (`messages/*`, `chat/*`, `models/*`).

### Non-goals

- No behavior change. On-wire bytes are identical, except the User-Agent default (§6.2),
  observable only if a consumer opts into the generic `try_new()` instead of supplying its
  own UA — and the consumers supply their own.
- No stacked HTTP-over-byte-transport (§3.4); no second concrete transport built now.
- No Tower adoption (§3.5); no merge of the SDKs' client/config/error layers.

## 5. Crate layout

```
crates/airs-transport/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs          # module decls + re-exports + crate docs
    ├── transport.rs    # Transport generic trait
    ├── http_transport.rs # HttpTransport marker sub-trait + blanket impl (named to avoid the extern `http` crate)
    ├── body.rs         # BodyStream alias
    ├── error.rs        # TransportError + is_retryable
    ├── collect.rs      # collect_body + MAX_RESPONSE_BODY_BYTES
    ├── reqwest_impl.rs # ReqwestTransport (impl Transport), classify, TLS, adapter
    └── mock.rs         # MockHttpTransport (mocks Transport) — feature __test-mocks
```

`lib.rs` public re-export surface:

```rust
pub use body::BodyStream;
pub use collect::{collect_body, MAX_RESPONSE_BODY_BYTES};
pub use error::TransportError;
pub use http_transport::HttpTransport;
pub use transport::Transport;

#[cfg(feature = "transport-reqwest")]
pub use reqwest_impl::ReqwestTransport;
#[cfg(feature = "__test-mocks")]
pub use mock::MockHttpTransport;
```

## 6. Public API decisions

### 6.1 Canonical source for divergent copies

The **`openrouter-rs`** `reqwest_impl.rs` is canonical: it factors the branch logic into a
pure, unit-tested `fn classify(elapsed, is_timeout, is_connect, is_tls, is_request, is_body,
msg) -> TransportError` plus a thin `classify_reqwest_error` wrapper. The extracted crate
adopts this form. `TransportError` is byte-identical in both crates and moves unchanged
(variants `Network`, `Tls`, `Timeout { elapsed }`, `BodyStream`, `Build`, `Other`;
`#[non_exhaustive]`; `is_retryable` true only for `Network` and `Timeout`).

### 6.2 User-Agent seam (resolved)

`ReqwestTransport` currently bakes a per-SDK UA into the `reqwest::Client`. The shared crate
cannot hardcode either provider's brand. Resolution:

- `ReqwestTransport::try_new()` → UA `airs-transport/<VERSION>`.
- `ReqwestTransport::try_new_with_user_agent(ua: &str)` → **new** constructor; builds a client
  with the caller-supplied UA; `TransportError::Build` on client-build failure.
- `ReqwestTransport::from_client(client)` → unchanged escape hatch.

Each consumer's client builder calls `try_new_with_user_agent(concat!("<crate>/",
env!("CARGO_PKG_VERSION")))`, so the on-wire UA each provider sees is unchanged.

### 6.3 `collect_body` availability

In `airs-transport`, `collect_body` depends only on `BodyStream` and `TransportError` (both
always present), so it is **unconditional** (no feature gate).

## 7. `airs-transport` Cargo manifest

```toml
[package]
name        = "airs-transport"
description = "Generic async transport substrate (with HTTP/reqwest layer) shared by airsstack SDK crates."
version     = "0.1.0"
# edition / rust-version / license / repository / authors / publish — inherited
readme      = "README.md"

[lints]
workspace = true

[features]
default           = []
transport-reqwest = ["dep:reqwest"]
__test-mocks      = ["dep:mockall"]

[dependencies]
http             = { workspace = true }
bytes            = { workspace = true }
futures-core     = { workspace = true }
pin-project-lite = { workspace = true }
async-trait      = { workspace = true }
thiserror        = { workspace = true }
reqwest          = { workspace = true, optional = true }
mockall          = { workspace = true, optional = true }

[dev-dependencies]
tokio        = { workspace = true, features = ["full", "test-util"] }
futures-util = { workspace = true }
```

Deliberately excluded: `serde`, `serde_json`, `url`, `secrecy`, `tracing`,
`eventsource-stream`. `default = []`; consumers forward `transport-reqwest` / `__test-mocks`.
A short `README.md` ships with the crate.

## 8. Consumer cutover (applied to both `clauders` and `openrouter-rs`)

1. **`Cargo.toml`** — add `airs-transport = { path = "../airs-transport" }`; rewrite
   `transport-reqwest = ["airs-transport/transport-reqwest"]` and
   `__test-mocks = ["airs-transport/__test-mocks"]`; remove the now-unused direct optional
   `reqwest` dependency. (`openrouter-rs` keeps `streaming`/`eventsource-stream`.)
2. **`lib.rs`** — delete `pub mod transport;` and the local `transport/` directory; add
   `pub use airs_transport as transport;` so `crate::transport::{Transport, HttpTransport,
   BodyStream, ReqwestTransport, MockHttpTransport}` all resolve to the new crate's
   root re-exports.
3. **`error.rs`** — delete the `TransportError` enum + its impl + tests; add
   `pub use airs_transport::TransportError;`. Keep the `Transport(#[from] TransportError)`
   arm on the top-level `Error` enum.
4. **`wire_helpers.rs`** — delete local `collect_body` / `MAX_RESPONSE_BODY_BYTES` (+ their
   tests); add `use airs_transport::{collect_body, MAX_RESPONSE_BODY_BYTES};`. Keep
   `decode_api_error_from_parts` / `parse_retry_after` / envelope structs.
5. **`client.rs`**
   - Replace `ReqwestTransport::try_new()` with
     `ReqwestTransport::try_new_with_user_agent(concat!("<crate>/", env!("CARGO_PKG_VERSION")))`.
   - **`DefaultTransportPlaceholder`** (the `#[cfg(not(feature = "transport-reqwest"))]`
     stand-in) currently does `impl HttpTransport for DefaultTransportPlaceholder { async fn
     send … }`. Because `HttpTransport` is now blanket-implemented, change this to
     `impl Transport for DefaultTransportPlaceholder` with the three associated types and the
     same panicking `send` body; the blanket impl then yields `HttpTransport`. Add `Transport`
     to the `use crate::transport::…` import.
6. **Code that calls `.send()` on a CONCRETE transport type** — add `Transport` to its
   `use` so the supertrait method resolves. **Amendment (discovered during T4):** the
   generic resource modules (`messages/resource.rs`, `models/resource.rs`,
   `messages/batches/resource.rs`, `chat/resource.rs`) do **not** need this — they call
   `.send()` on a generic `T: HttpTransport` parameter, and the `async_trait`-generated
   code resolves the supertrait method through the bound without `Transport` in scope. The
   files that genuinely need `Transport` imported are those calling `.send()` on a
   concrete transport, namely the **integration tests**. In `clauders` these were
   `tests/transport_reqwest.rs` and `tests/transport_mock.rs` (each swapped its unused
   `HttpTransport` import for `Transport`). For `openrouter-rs`, let the compiler identify
   them (unresolved-method `send` → add `Transport` to that file's import). Mock-only test
   modules that call `expect_send()` need no change.
7. Delete the moved `transport/` source files from each crate.

## 9. Error-handling strategy

Unchanged from the caller's perspective: the transport returns `Result<_, TransportError>`
and treats HTTP 4xx/5xx as `Ok`; each consumer's top-level `Error` keeps its
`Transport(#[from] TransportError)` arm; `TransportError::is_retryable` moves with the enum;
`try_new_with_user_agent` maps a build failure to `TransportError::Build`.

## 10. Testing strategy

- **`airs-transport` owns the transport unit tests:** the classifier, TLS-message detection,
  `collect_body` (within/over limit), `TransportError::is_retryable`, and the
  `ReqwestTransport` constructors all pass under `cargo test -p airs-transport`.
- **Blanket-impl test:** a unit test defines a trivial `Transport` impl with the HTTP
  associated types and asserts (by using it where `T: HttpTransport` is required) that the
  blanket impl makes it an `HttpTransport`. This guards the layering.
- **Feature matrix:** `cargo hack --each-feature -p airs-transport` passes, exercising
  `transport-reqwest` and `__test-mocks` independently and the `default = []` base
  (`transport`/`http`/`body`/`error`/`collect` with no optional deps).
- **Consumer regression:** after cutover each consumer's existing suite (unit + integration +
  `wiremock` in clauders) passes unchanged, proving the re-export shims, the `#[from]`
  conversion, the supertrait `send` resolution, and the `Transport`-based placeholder all hold.
- **Doctests:** moved doc examples are rewritten to `airs_transport::` paths and pass
  `cargo test --doc -p airs-transport`.

## 11. Definition of Done

- `cargo fmt --check` clean.
- `cargo build --workspace` green.
- `cargo clippy --workspace --all-targets` green under workspace lints.
- `cargo test --workspace` green (unit, integration, doctests, all three crates).
- `cargo hack --each-feature` green for `airs-transport`, `clauders`, `openrouter-rs`.

## 12. Migration order (low-risk sequencing)

1. Scaffold `airs-transport`; build the layered substrate; green it standalone. The SDKs are
   untouched and still build.
2. Cut over `clauders`; green it (suite + feature matrix).
3. Cut over `openrouter-rs`; green it (suite + feature matrix).
4. Full workspace DoD gate.

Each step leaves the tree compiling.

## 13. Commit plan

Conventional Commits with workspace-aware scopes:

- `feat(airs-transport): add layered transport substrate crate`
- `refactor(clauders): consume airs-transport for the transport layer`
- `refactor(openrouter-rs): consume airs-transport for the transport layer`

The workspace-member edit rides with the first commit (a buildable crate must exist before
the root manifest lists it).

## 14. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| `async_trait` + associated types + blanket impl + `mockall` interaction is fiddly. | The blanket-impl test (§10) and the green feature matrix prove the layering compiles; `mockall` supports associated types declared concretely in the `mock!` impl block. |
| A resource module calls `t.send()` without `Transport` in scope after the method moved to the supertrait. | §8.6 enumerates the five resource files needing the import; the green build pins any missed site (unresolved-method error). |
| The `DefaultTransportPlaceholder` no-reqwest path breaks under the blanket impl. | §8.5 converts it to `impl Transport`; `cargo hack --each-feature` exercises the `not(transport-reqwest)` configuration. |
| A consumer references a transport item by a path the re-export shim does not preserve. | The shim re-exports the whole crate as `transport`, preserving every root-exported path; the green consumer build is the proof. |
| Pruning `reqwest` from a consumer breaks a stray reference. | Only `reqwest` is pruned; clippy + the green build confirm no other use remains. |

## 15. Future direction (not built here)

The `Transport` trait is the seam for a second concrete transport (non-`reqwest` HTTP, or a
non-HTTP transport implementing `Transport` with its own associated types) and for a later
migration to `tower::Service` should middleware become a requirement. None is in scope now;
this section records why the layering exists.
