# Architecture

Every call in this crate travels the same four layers. Nothing skips a layer,
and no layer knows about the one two steps down.

```
Client<T>                 handle; Arc<ClientInner<T>> { config, transport, auth }; Clone = refcount
  └─ .chat() / .models()  short-lived resource handle borrowing &Client, created at the call site
       └─ resource.rs     serialize → join URL → set headers → dispatch → interpret status
            └─ airs-transport   HttpTransport::send, BodyStream, collect_body, ReqwestTransport
```

## The handle is a refcount, not a connection

`Client<T>` holds an `Arc<ClientInner<T>>` carrying three things: `Config`
(base URL, attribution headers, timeout), the transport, and `Auth`. Cloning is
an `Arc::clone`. Spawn a hundred tasks with a hundred clones and they share one
transport and one connection pool.

This is why `Client` is cheap and why there is no separate "connection" concept
to manage. `ref_count()` exists so you can confirm your sharing model is what you
think it is.

## Resources are borrows, not objects

`client.chat()` returns a `ChatResource<'_, T>` holding a `&Client<T>` and
nothing else. It is created at the call site, used for one call, and dropped. It
is not a builder, does not accumulate state, and is not worth storing.

The reason for the layer at all is that endpoint-specific dispatch has to live
*somewhere*, and putting it on `Client` would grow the handle for every
endpoint. The crate has two, `chat` and `models`, and each is self-contained:
one resource module and one method on `Client`, with nothing else touched.

## The transport is a generic parameter, never a trait object

```rust
pub struct Client<T> where T: HttpTransport
```

Not `Box<dyn HttpTransport>`. The consequence is that a client's transport is
part of its type: `Client<ReqwestTransport>` and `Client<MyFake>` are different
types, and functions that take a client should be generic:

```rust
async fn ask<T: HttpTransport>(client: &Client<T>, q: &str) -> Result<String, Error>
```

What this buys:

- **Testability with no runtime cost.** Substituting a fake transport is a type
  parameter, not a virtual call. The seam monomorphises away.
- **No object-safety constraints** on the `Transport` trait, which is free to use
  associated types — and it does: `Request`, `Response`, `Error`.
- **No vtable and no allocation** on the hot path.

What it costs: the type parameter is visible, and it leaks into the signature of
anything holding a client. `DefaultClient` exists to soften that for callers who
only ever use `reqwest`.

There is exactly one trait object in the stack, `BodyStream`
(`Pin<Box<dyn Stream<…>>>`), because heterogeneous concrete body streams have to
be stored uniformly and there is no alternative.

### `HttpTransport` is a marker

`Transport` is the general contract: send one request, get one response, with
associated `Request` / `Response` / `Error` types and no HTTP vocabulary.
`HttpTransport` is a sub-trait with a blanket impl — any `Transport` whose
associated types are `Request<Bytes>`, `Response<BodyStream>`, and
`TransportError` *is* an `HttpTransport`, automatically. You never implement it;
you implement `Transport` and it follows.

Note that a 4xx or 5xx response is **not** a transport error. The transport
delivers it faithfully; interpreting the status is the resource layer's job.

## Where each concern lives

| Concern | Home | Deliberately not there |
|---|---|---|
| Serialising the request body | `chat::resource` | not the wire-format types |
| Joining base URL + path | `chat::resource` via `BaseUrl::join` | `BaseUrl` only guarantees the base is well-formed |
| Setting headers | `chat::resource`, names in `headers` | `Config` carries values, not header syntax |
| Sending bytes | `airs_transport` | knows no OpenRouter concept |
| Draining a body | `airs_transport::collect_body` | not `wire_helpers` |
| Classifying a non-2xx body | `wire_helpers::decode_api_error_from_parts` | shared by both endpoints, so neither duplicates it |
| Driving SSE | `chat::stream` | isolates `eventsource-stream` from the non-streaming path |
| Retry / backoff | **nowhere** | see [errors and retries](errors-and-retries.md) |

`wire_helpers` exists precisely so that adding a third endpoint does not mean a
third copy of the error-routing ladder.

## `mod.rs` and `lib.rs` are tables of contents

Every `mod.rs` and `lib.rs` in this crate contains module documentation, `mod`
declarations, and `pub use` re-exports — and nothing else. Implementation lives
in a sibling file named after the item it defines: `ChatStream` in `stream.rs`,
`CacheControl` in `cache_control.rs`, `decode_api_error_from_parts` in
`wire_helpers.rs`.

The payoff is navigational. Reading `chat/mod.rs` tells you what exists in about
sixty lines, and the file name tells you where to go next. `prelude.rs` is the
one glob-import surface and holds no items of its own.

## The crate is featureless

No `[features]` table in `Cargo.toml`, and no `cfg(feature = ...)` anywhere in
`src/`, `tests/`, or `examples/`. Streaming, tools, structured outputs, provider
routing, caching, and the model catalog are always compiled;
`cargo build --all-features` is byte-identical to `cargo build`.

The `mockall` test double lives in a `#[cfg(test)]`-only module rather than
behind a feature, which is why it is unavailable downstream — see
[how-to/test-with-a-mock-transport.md](../how-to/test-with-a-mock-transport.md).

## Boundaries the crate keeps

- **No foreign error type on the public surface.** `reqwest` failures become
  `TransportError`; `url::Url` stays private inside `BaseUrl`. A version bump in
  either dependency is not a breaking change here.
- **No `unsafe`.** `#![forbid(unsafe_code)]` at the crate root.
- **No coupling to sibling crates.** `openrouter-rs` is an independent SDK; no
  other crate in the workspace depends on it, and it depends only on
  `airs-transport`.
