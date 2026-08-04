# Reference — Errors

Three types, layered by when the failure happens.

| Type | Domain |
|---|---|
| `TransportError` | wire-level failures (re-exported from `airs-transport`) |
| `BuildError` | client construction, before any request exists |
| `Error` | the top-level wrapper every fallible public call returns |

All three are `#[non_exhaustive]`.

## `Error`

```rust
pub enum Error { /* #[non_exhaustive] */ }
```

| Variant | Fields | Raised when |
|---|---|---|
| `Transport` | `TransportError` | network, TLS, timeout, body framing. `#[from]` |
| `Api` | `status: u16`, `code: i32`, `message: String`, `metadata: Option<Value>` | non-2xx with a decodable envelope, not otherwise routed |
| `Moderation` | `reasons: Vec<String>`, `flagged_input: String`, `provider_name: String`, `model_slug: String` | HTTP 403 with moderation metadata |
| `Provider` | `provider_name: String`, `raw: Value` | upstream provider error passed through |
| `RateLimit` | `retry_after: Option<Duration>` | HTTP 429 |
| `UndecodableApiError` | `status: u16`, `detail: String` | non-2xx whose body is not a recognised envelope |
| `Stream` | `String` | SSE framing failure, mid-stream interruption, or a mid-stream error event |
| `Serde` | `context: &'static str`, `source: serde_json::Error` | encode/decode failure inside the SDK |
| `InvalidRequest` | `String` | rejected before the network: URL join or HTTP request construction |
| `Build` | `BuildError` | client construction failed. `#[from]` |

`Serde::context` names the type involved: `"ChatRequest"`, `"ChatCompletion"`,
`"StreamChunk"`, `"ModelsResponse"`.

### `is_retryable()`

```rust
pub const fn is_retryable(&self) -> bool
```

| Variant | Retryable |
|---|---|
| `Transport(e)` | delegates to `TransportError::is_retryable()` |
| `RateLimit` | ✅ |
| `Api { status, .. }` | ✅ for 408, 500, 502, 503; ❌ otherwise |
| `Moderation`, `Provider`, `UndecodableApiError`, `Stream`, `Serde`, `InvalidRequest`, `Build` | ❌ |

### `retry_after()`

```rust
pub const fn retry_after(&self) -> Option<Duration>
```

Returns the value only for `RateLimit`; `None` for every other variant.

## `BuildError`

| Variant | Message | Constructed |
|---|---|---|
| `Transport(String)` | `transport construction failed: …` | ✅ — the only one, from `Client::builder()` when `reqwest::Client` cannot initialise |
| `BaseUrl(String)` | `invalid base URL: …` | ❌ — `BaseUrl::parse` returns `InvalidBaseUrl` instead |
| `InvalidConfig(String)` | `invalid config: …` | ❌ |

`ClientBuilder::build()` returns `Result<_, BuildError>` but cannot fail: every
value it holds was validated at construction. Propagate it with `?` anyway —
`Error` carries `#[from] BuildError`, and `BuildError` is `#[non_exhaustive]`.

## `TransportError`

Re-exported as `openrouter_rs::error::TransportError`.

| Variant | Cause | Retryable |
|---|---|---|
| `Network(String)` | connection refused, reset, DNS | ✅ |
| `Timeout { elapsed: Duration }` | request exceeded its deadline | ✅ |
| `Tls(String)` | handshake or certificate failure | ❌ |
| `BodyStream(String)` | body failure after headers, or the 16 MiB cap exceeded | ❌ |
| `Build(String)` | outgoing request could not be constructed | ❌ |
| `Other(String)` | uncategorised — treated as unsafe to retry | ❌ |

`reqwest` errors are classified into these variants at the transport boundary,
in this order: timeout → connect → TLS → request-build → body → other. No
`reqwest` type reaches the public surface, so a `reqwest` version bump is not a
breaking change.

## How a non-2xx response is classified

Every non-2xx response, on both endpoints, goes through the same decoder.

```
1. Read `Retry-After`. Parse as an integer number of seconds.
   Anything else (including the HTTP-date form) → None.

2. Try to decode the body as {"error": {"code": i32, "message": String,
   "metadata"?: Value}}.
   ├─ fails → Error::UndecodableApiError { status, detail: <raw body> }
   └─ succeeds → continue

3. status == 429                              → Error::RateLimit { retry_after }

4. status == 403 AND metadata decodes as
   {reasons, flagged_input, provider_name,
    model_slug}                               → Error::Moderation { .. }

5. metadata decodes as {provider_name, raw}   → Error::Provider { .. }
   (any status — this is checked after the 403 moderation case)

6. otherwise                                  → Error::Api { status, code,
                                                  message, metadata }
```

The moderation and provider cases are distinguished purely by the *shape* of the
`metadata` object, not by status alone. A 403 without moderation metadata falls
through to step 5 and then step 6.

`detail` on `UndecodableApiError` is the raw body as
`String::from_utf8_lossy` — an HTML error page from a proxy, typically.

## What `Error` does not absorb

The newtype rejection errors (`InvalidApiKey`, `InvalidModelId`,
`InvalidFunctionName`, `InvalidSchemaName`, `InvalidProviderSlug`,
`InvalidToolCallId`, `InvalidBaseUrl`, `InvalidMaxTokens`, `InvalidTemperature`,
`InvalidTopP`, `InvalidFrequencyPenalty`, `InvalidPresencePenalty`,
`InvalidRepetitionPenalty`, `InvalidStopSequences`, `InvalidPrice`,
`InvalidPricePerToken`, `InvalidCacheTtlSeconds`, `InvalidThroughputFloor`,
`InvalidLatencyCeiling`) implement `std::error::Error` but are **not** variants
of `Error` and do not convert with `?`.

That is deliberate. They are input errors caught before a request exists, not
API failures. Map them into your own error type at the construction site, or use
`Box<dyn std::error::Error>` as the examples do.

## Two failure points on a streaming call

`ChatResource::stream` returns `Err` for anything that goes wrong before the
first body byte, including every non-2xx status — those are classifiable and
possibly retryable. Errors yielded *by* the `ChatStream` are terminal: the next
poll returns `None`. See [streaming.md](streaming.md#termination-rules).
