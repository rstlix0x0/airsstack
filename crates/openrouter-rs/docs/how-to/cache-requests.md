# How to cache requests

Two unrelated caches sit in front of your request. Pick the right one before you
write any code — see [the two caches](../explanation/the-two-caches.md) if you
are not sure which you want.

| | Provider **prompt cache** | Gateway **edge cache** |
|---|---|---|
| Lives in | request **body** | request/response **headers** |
| Caches | a prefix of the prompt, upstream at the provider | the whole response, at OpenRouter |
| You set | `cache_control` breakpoints | `ResponseCache` on `send_cached` |
| You observe | `usage.prompt_tokens_details` | `Cached { status, age_secs, ttl_secs }` |

You can use both at once. They do not interact.

## Prompt cache: mark a cacheable prefix

Attach a `cache_control` breakpoint to the content part that ends your stable
prefix. Everything up to and including that part becomes cacheable upstream.

```rust
use openrouter_rs::prelude::*;
use openrouter_rs::chat::ContentPart;

let long_instructions = std::fs::read_to_string("system_prompt.txt")?;

let system = Message::system(vec![
    ContentPart::text_cached(long_instructions, CacheControl::ephemeral()),
]);

let req = ChatRequest::builder()
    .model(ModelId::custom("anthropic/claude-sonnet-4-5")?)
    .messages(vec![system, Message::user("First question.")])
    .build();
```

`ContentPart::text(..)` is the same part without a breakpoint. A `Message` built
from a plain `&str` produces the bare-string content form, which has nowhere to
hang a breakpoint — use the `Vec<ContentPart>` form when you need one.

### Choose a TTL

```rust
CacheControl::ephemeral()                       // {"type":"ephemeral"} — provider default
CacheControl::with_ttl(CacheTtl::FiveMinutes)   // {"type":"ephemeral","ttl":"5m"}
CacheControl::with_ttl(CacheTtl::OneHour)       // {"type":"ephemeral","ttl":"1h"}
```

### Top-level form

Some providers (Anthropic direct) accept a single breakpoint for the whole
request instead of per-part markers:

```rust
let req = ChatRequest::builder()
    .model(model)
    .messages(messages)
    .cache_control(CacheControl::with_ttl(CacheTtl::OneHour))
    .build();
```

### Confirm it worked

The prompt cache reports through the usage block, not through headers:

```rust
if let Some(d) = completion.usage.as_ref().and_then(|u| u.prompt_tokens_details) {
    println!("cache reads:  {:?}", d.cached_tokens);
    println!("cache writes: {:?}", d.cache_write_tokens);
}
if let Some(discount) = completion.usage.as_ref().and_then(|u| u.cache_discount) {
    // negative on the write that established the entry, positive on reads
    println!("cache discount: {discount}");
}
```

## Edge cache: reuse a whole response

Switch from `send` to `send_cached` and pass a `ResponseCache`.

```rust
let cached = client
    .chat()
    .send_cached(req, ResponseCache::enabled())
    .await?;

println!("status={:?} age={:?}s ttl={:?}s", cached.status, cached.age_secs, cached.ttl_secs);

let completion = cached.value;   // the ChatCompletion itself
```

`Cached<T>` is a plain envelope: `value`, `status`, `age_secs`, `ttl_secs`.

### Control the entry

```rust
let control = ResponseCache::enabled()
    .ttl_secs(600)?     // 1..=86400, rejected outside that range
    .clear();           // force-refresh: discard any existing entry
```

| Builder | Header sent |
|---|---|
| `ResponseCache::enabled()` | `x-openrouter-cache: true` |
| `ResponseCache::disabled()` | `x-openrouter-cache: false` |
| `.ttl_secs(n)` | `x-openrouter-cache-ttl: n` |
| `.clear()` | `x-openrouter-cache-clear: true` |

`ttl_secs` returns `Result` because the range is validated; the other two are
infallible. Without `.clear()` no clear header is sent at all.

### Read the outcome

`status` decodes from `x-openrouter-cache-status`. The match is
case-insensitive on `HIT`; **anything else, including an absent header, is
`CacheStatus::Miss`.** A miss therefore does not distinguish "computed fresh"
from "caching was off".

`age_secs` (`x-openrouter-cache-age`) is populated on hits. `ttl_secs`
(`x-openrouter-cache-ttl`) carries the remaining or declared lifetime. Both are
`None` when the header is absent or is not an integer.

### Streaming works too

```rust
let mut cached = client
    .chat()
    .stream_cached(req, ResponseCache::enabled())
    .await?;

println!("cache: {:?}", cached.status);          // known before any token arrives
while let Some(chunk) = cached.value.next().await { /* ... */ }
```

The cache headers arrive with the response headers, so you learn the outcome
before the first chunk.

## Make an edge-cache hit actually happen

The gateway will not cache a trivially small request. `examples/04_caching.rs`
sends a deliberately long system prompt twice and prints the status of each:

```bash
OPENROUTER_API_KEY=sk-... cargo run --example 04_caching
```

Expect `Miss` then `Hit`. If both are misses, the request was probably below the
gateway's minimum cacheable size — grow the prompt.

## Gotcha: two things named TTL

`CacheTtl` (`5m` / `1h`) is the **prompt** cache. `CacheTtlSeconds` (`1..=86400`)
is the **edge** cache. They are unrelated types with unrelated ranges.
