# Reference — Caching

Two independent caches. See [the two caches](../explanation/the-two-caches.md)
for why they are separate, and
[how-to/cache-requests.md](../how-to/cache-requests.md) for usage.

| | Prompt cache | Edge cache | Usage stats |
|---|---|---|---|
| Module | `chat::cache_control` | `chat::response_cache` + `chat::cached` | `chat::token_details` |
| Location | request body | request/response headers | response body |
| Types | `CacheControl`, `CacheKind`, `CacheTtl` | `ResponseCache`, `CacheMode`, `CacheClear`, `CacheTtlSeconds`, `Cached<T>`, `CacheStatus` | `PromptTokensDetails`, `CompletionTokensDetails`, `CostDetails` |

---

## Prompt cache (request body)

### `CacheControl`

```rust
pub struct CacheControl {
    pub kind: CacheKind,          // serialised under the key "type"
    pub ttl: Option<CacheTtl>,    // omitted when None
}
```

| Constructor | Serialises to |
|---|---|
| `CacheControl::ephemeral()` | `{"type":"ephemeral"}` |
| `CacheControl::with_ttl(ttl)` | `{"type":"ephemeral","ttl":"…"}` |

Both are `const`. Derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`,
`Serialize`, `Deserialize`.

### `CacheKind`

One variant: `Ephemeral` → `"ephemeral"`.

### `CacheTtl`

| Variant | Wire |
|---|---|
| `FiveMinutes` | `"5m"` |
| `OneHour` | `"1h"` |

Absent means the provider default.

### Attach points

| Where | How |
|---|---|
| A message content part | `ContentPart::text_cached(text, cache_control)` |
| The whole request | `ChatRequestBuilder::cache_control(cc)` → top-level `cache_control` key |

A message built from a plain `&str` uses the bare-string content form, which has
no slot for a breakpoint. Use the `Vec<ContentPart>` form.

---

## Edge cache (headers)

### `ResponseCache`

Not serialised into the body. The resource layer reads it and renders request
headers.

| Constructor / method | Signature | Header effect |
|---|---|---|
| `ResponseCache::enabled()` | `const fn () -> Self` | `x-openrouter-cache: true` |
| `ResponseCache::disabled()` | `const fn () -> Self` | `x-openrouter-cache: false` |
| `ttl_secs(u32)` | `const fn (self, u32) -> Result<Self, InvalidCacheTtlSeconds>` | `x-openrouter-cache-ttl: <n>` |
| `clear()` | `const fn (self) -> Self` | `x-openrouter-cache-clear: true` |

Accessors: `mode() -> CacheMode`, `ttl() -> Option<CacheTtlSeconds>`,
`clear_directive() -> CacheClear`. The type is `Copy`.

The `x-openrouter-cache` header is always sent when a `ResponseCache` is
supplied. The TTL header is sent only when a TTL was set. The clear header is
sent only for `CacheClear::Clear` — `Keep` sends nothing.

### `CacheMode` / `CacheClear`

`CacheMode`: `Enabled` | `Disabled`. `CacheClear`: `Clear` | `Keep`.
Neither is serialised; both drive header rendering.

### `CacheTtlSeconds`

Validated `u32` in `1..=86400` (one second to one day). `new` is `const` and
returns `Result<Self, InvalidCacheTtlSeconds>`; `get() -> u32`.

`InvalidCacheTtlSeconds` is a unit struct:
`"cache TTL must be within 1..=86400 seconds"`.

> Not to be confused with `CacheTtl` (`5m` / `1h`), which belongs to the prompt
> cache.

### `Cached<T>`

```rust
pub struct Cached<T> {
    pub value: T,
    pub status: CacheStatus,
    pub age_secs: Option<u32>,
    pub ttl_secs: Option<u32>,
}
```

Returned as `Cached<ChatCompletion>` from `send_cached` and
`Cached<ChatStream>` from `stream_cached`.

### `CacheStatus`

`Hit` | `Miss`. `CacheStatus::from_header_value(&str)` matches `HIT`
case-insensitively; **everything else maps to `Miss`**, including an absent or
unrecognised header. A `Miss` therefore does not distinguish "computed fresh"
from "caching was disabled".

### Response header decoding

| Header | Field | Parse |
|---|---|---|
| `x-openrouter-cache-status` | `status` | case-insensitive `HIT`, else `Miss` |
| `x-openrouter-cache-age` | `age_secs` | `u32`; `None` if absent or not an integer |
| `x-openrouter-cache-ttl` | `ttl_secs` | `u32`; `None` if absent or not an integer |

Note the TTL header name is used in both directions — as a request directive and
as a response report.

---

## Usage statistics (response body)

The prompt cache reports through `Usage`, not through headers.

| Field | Meaning |
|---|---|
| `usage.prompt_tokens_details.cached_tokens` | tokens served from the prompt cache (reads) |
| `usage.prompt_tokens_details.cache_write_tokens` | tokens written on the request that established the entry |
| `usage.cache_discount` | cost delta: negative on a write, positive on reads |

All are `Option`; a provider that does not report them yields `None`.

The edge cache reports nothing in the body — its outcome is entirely in the
`Cached<T>` envelope.
