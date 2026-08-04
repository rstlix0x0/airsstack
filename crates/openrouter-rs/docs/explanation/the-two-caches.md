# The two caches

The single most confusable thing in this crate is that "caching" means two
entirely unrelated mechanisms, sitting at different layers, controlled by
different types, reported through different channels. They share a word and
nothing else.

```
                      your request
                            │
        ┌───────────────────┴───────────────────┐
        │        OpenRouter gateway             │
        │                                       │
        │   EDGE CACHE  ← headers               │   caches the whole response
        │   x-openrouter-cache: true            │   keyed by the request
        └───────────────────┬───────────────────┘
                            │
        ┌───────────────────┴───────────────────┐
        │        upstream provider              │
        │                                       │
        │   PROMPT CACHE  ← request body        │   caches a prompt prefix
        │   "cache_control": {...}              │   reused across requests
        └───────────────────────────────────────┘
```

## What each one actually caches

The **prompt cache** is a provider feature. You mark a prefix of the prompt as
cacheable and the provider keeps its computed representation warm. The next
request that shares that exact prefix skips recomputing it. The response is
still generated fresh — only the prefix work is saved. This is what makes a long
system prompt cheap to reuse.

The **edge cache** is an OpenRouter feature. It stores the entire response and
serves it again for a matching request, without touching a provider at all. No
generation happens; you get the identical bytes back.

They compose. A request can hit the edge cache (no provider call) or miss it and
then hit the provider's prompt cache (cheaper generation).

## Where each one lives in the code

| | Prompt cache | Edge cache |
|---|---|---|
| Module | `chat::cache_control` | `chat::response_cache` + `chat::cached` |
| Travels in | request **body** | request and response **headers** |
| Control type | `CacheControl` | `ResponseCache` |
| Serialised into the request? | ✅ yes, it is a body field | ❌ never — the resource layer renders headers from it |
| Attached via | `ContentPart::text_cached`, or `ChatRequestBuilder::cache_control` | `send_cached` / `stream_cached` |
| Outcome reported in | response **body**, under `usage` | response **headers**, wrapped as `Cached<T>` |
| TTL type | `CacheTtl` (`5m` \| `1h`) | `CacheTtlSeconds` (`1..=86400`) |
| Read via | `usage.prompt_tokens_details`, `usage.cache_discount` | `Cached { status, age_secs, ttl_secs }` |

The `send` / `send_cached` split exists solely for the edge cache. Prompt caching
needs no separate method, because it is just another field in the body — you can
use it on all four call paths.

## Why the crate keeps them apart

The temptation is to unify: one `Cache` type, one `.cache(...)` setter, and let
the SDK sort out where each piece goes. That would be wrong for three reasons.

**They mean different things.** A prompt-cache hit still bills you for
generation; an edge-cache hit does not. Collapsing them would hide a real cost
difference behind one word.

**They have different lifetimes and ranges.** `5m` / `1h` versus `1..=86400`
seconds. A unified TTL would have to be the union, and then reject half its
values depending on which cache you meant.

**They fail differently.** A provider that ignores `cache_control` silently
degrades to no caching. The edge cache always reports a status header. One is
best-effort, one is observable.

Three separate modules, three separate type families, and a warning in the crate
guidance not to conflate them, is the response to all of that.

## The `Miss` that is not a miss

`CacheStatus::from_header_value` maps `HIT` (case-insensitive) to `Hit` and
**everything else** to `Miss` — including an absent header and an unrecognised
value.

So `CacheStatus::Miss` means "not a confirmed hit". It does not distinguish:

- computed fresh because nothing was cached
- computed fresh because you passed `ResponseCache::disabled()`
- the gateway did not report a status at all

If that distinction matters to your metrics, track what you sent alongside what
came back. The envelope alone will not tell you.

## The third thing named "cache"

`chat::token_details` is a third module with "cache" in its vocabulary, and it
is neither control surface — it is the *statistics* the prompt cache reports:

```rust
usage.prompt_tokens_details.cached_tokens        // reads
usage.prompt_tokens_details.cache_write_tokens   // writes
usage.cache_discount                             // negative on write, positive on read
```

The sign convention on `cache_discount` is worth internalising: the request that
*establishes* a cache entry costs extra, so the discount is negative. The reads
that follow are where you get it back.

## Practical guidance

Reach for the **prompt cache** when the same long prefix appears in many
different requests — a system prompt, a document, a few-shot block — and the
suffix varies.

Reach for the **edge cache** when the *same complete request* repeats and a
stale-but-identical answer is acceptable.

If neither describes your traffic, you want neither. See
[how-to/cache-requests.md](../how-to/cache-requests.md) for the mechanics of
each.
