# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Scope: the `openrouter-rs` crate. The workspace root `../../CLAUDE.md` still applies — this file adds
only what is specific to this crate and does not repeat the root's rules.

## What this crate is

An unofficial Rust SDK for the [OpenRouter API](https://openrouter.ai/docs) — a unified,
OpenAI-compatible gateway that routes chat completions across many providers behind one API key. It
is **independent of `clauders`**: no `clauders` code depends on it, and its former runtime
integration was severed (root CLAUDE.md, vision §9.1). Do not re-couple them.

Two endpoints are implemented: `POST /chat/completions` (`ChatResource::send`, `send_cached`,
`stream`, `stream_cached`) and `GET /models` (`ModelsResource::list`).

## Documentation

`docs/` is a Diátaxis tree — tutorials, how-to guides, reference, explanation — indexed by
`docs/README.md`. Everything in it describes the crate as implemented.

Reader-facing text (`docs/`, `README.md`, rustdoc in `src/`, `examples/`) must not carry
development-process vocabulary: no phases, plans, specs, roadmaps, "not yet", "currently",
"reserved for a future X", no internal tooling or session references, no sections addressed to
maintainers. State observable behaviour without the timeline. Absent surface is documented as
absent, never as pending. Full rule with the rewrite table and the pre-commit grep:
[`.claude/rules/documentation-voice.md`](.claude/rules/documentation-voice.md).

## Commands

The workspace Definition of Done, scoped to this crate while developing here:

```bash
cargo fmt --all -- --check
cargo clippy -p openrouter-rs --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p openrouter-rs --all-features --no-deps
cargo test  -p openrouter-rs --all-targets --all-features
cargo test  -p openrouter-rs --all-features --doc
```

Run one unit test by its module path (tests are colocated `#[cfg(test)] mod tests`):

```bash
cargo test -p openrouter-rs --lib config::tests::default_points_at_openrouter
cargo test -p openrouter-rs --doc chat::cache_control::CacheKind   # one doctest
```

Compile-fail fixtures under `tests/compile_fail/` are golden files driven by `trybuild`. After an
intentional change to a type-state signature, regenerate the expected diagnostics rather than
hand-editing them (`trybuild-1.0.116/src/lib.rs:127`):

```bash
TRYBUILD=overwrite cargo test -p openrouter-rs --test builder_compile
```

Examples hit the live API and read the key from the environment:

```bash
OPENROUTER_API_KEY=sk-... cargo run -p openrouter-rs --example 01_chat
```

## Architecture

Every call travels the same four layers. Nothing skips a layer.

```
Client<T>                 handle; Arc<ClientInner<T>> { config, transport, auth }; Clone = refcount
  └─ .chat() / .models()  short-lived resource handle borrowing &Client, created at the call site
       └─ resource.rs     serialize → join URL → set headers → dispatch → interpret status
            └─ airs-transport   HttpTransport::send, BodyStream, collect_body, ReqwestTransport
```

The transport is a **generic parameter, never a trait object** — `Client<T: HttpTransport>`, with
`DefaultClient = Client<ReqwestTransport>`. Tests substitute `test_support::MockHttpTransport`
(a `mockall` double, `cfg(test)` only) through `Client::builder_with_transport`.

Four conventions carry most of the design:

**Type-state builders for required fields.** `ClientBuilder<Key, T>` only exposes `build()` once
`Key = Present` (`api_key` supplied); `ChatRequestBuilder<M, Ms>` only once both `model` and
`messages` are `Present`. There is no runtime "missing field" error to add — a missing field is a
compile error, proven by the `trybuild` fixtures. All mutable builder data lives in one private
non-generic struct (`ClientBuilderFields`, `ChatRequestFields`) so a state transition moves the whole
value; adding a field must not touch the transition code.

**Parse, don't validate, at the newtype boundary.** `src/types/` holds the validated domain values —
`ApiKey` (secret, `secrecy`-backed), `BaseUrl`, `ModelId`, `FunctionName`, `ProviderSlug`,
`SchemaName`, `ToolCallId`, bounded sampling numerics (`Temperature`, `TopP`, `MaxTokens`, …),
`StopSequences`, `Price`, `PricePerToken`. Each exports its `Invalid*` failure reason alongside it.
Request-building code downstream never re-checks these invariants. New API surface that takes a
string or a number gets a newtype, not a primitive.

**No foreign error type in the public surface.** `error::Error` is the single top-level wrapper;
`reqwest` failures are converted to `airs_transport::TransportError` at the transport boundary, so a
`reqwest` bump is not a breaking change. Non-2xx bodies are routed by
`wire_helpers::decode_api_error_from_parts` into the rate-limit / moderation / provider-passthrough /
generic-API / undecodable cases; `Error::is_retryable` is the caller's retry signal (the SDK itself
carries no retry layer). `BuildError` covers what fails before a request is sent.

**`mod.rs` / `lib.rs` are export-only.** Module docs plus `mod` / `pub use`, nothing else.
Implementation lives in a sibling file named after the item. `src/prelude.rs` is the one glob-import
surface; it holds no items of its own.

### Two distinct caches — do not conflate

| Cache | Where it lives | Types |
|---|---|---|
| Provider **prompt cache** | request *body* (`cache_control` breakpoints, per-message or top-level) | `chat/cache_control.rs` — `CacheControl`, `CacheKind`, `CacheTtl` |
| Gateway **edge cache** | request/response *headers* (`X-OpenRouter-Cache*`) | `chat/response_cache.rs` + `chat/cached.rs` — `ResponseCache`, `Cached<T>`, `CacheStatus` |
| Cache **usage stats** | response body | `chat/token_details.rs` |

`send_cached` / `stream_cached` are the edge-cache variants; they return a `Cached<T>` envelope
carrying the hit/miss outcome. `send` / `stream` are the plain paths.

## Gotchas

- **The crate is featureless.** No `[features]` in `Cargo.toml`, no `cfg(feature = ...)` anywhere in
  `src/`, `tests/`, or `examples/`; `--all-features` equals the default build. Streaming, tools,
  structured outputs, provider routing, caching, and the model catalog are always compiled. Doc
  comments that describe a capability as gated are stale — the last of them were removed; do not
  reintroduce that framing.
- **`BaseUrl::join` is additive**, so endpoint path constants carry no leading slash (`CHAT_PATH =
  "chat/completions"`) and the default base URL ends in `/` (`https://openrouter.ai/api/v1/`).
- **`ChatStream` is terminal on error**: once it yields `Err`, the next poll returns `None`. A
  `data: [DONE]` line ends it cleanly.
- Examples are auto-discovered from `examples/`, but this manifest lists each one explicitly under
  `[[example]]`. Keep the convention when adding one.
