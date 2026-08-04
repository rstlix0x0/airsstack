# How to steer provider routing

OpenRouter picks a provider for you. `ProviderPreferences` tells it how.
Everything is optional; an empty preferences object serialises to `{}` and
changes nothing.

```rust
use openrouter_rs::prelude::*;

let prefs = ProviderPreferences::builder()
    .sort(ProviderSort::Price)
    .allow_fallbacks(FallbackPolicy::Deny)
    .build();

let req = ChatRequest::builder()
    .model(ModelId::custom("openai/gpt-4o-mini")?)
    .messages(vec![Message::user("hi")])
    .provider(prefs)
    .build();
```

This is a plain fluent builder, not a type-state one — nothing is required, so
`build()` is always callable and infallible.

## Pick specific providers

Three list setters, each taking validated `ProviderSlug` values (non-empty, no
whitespace):

```rust
use openrouter_rs::types::ProviderSlug;

let prefs = ProviderPreferences::builder()
    .order(vec![ProviderSlug::new("anthropic")?, ProviderSlug::new("openai")?])
    .ignore(vec![ProviderSlug::new("some-provider")?])
    .build();
```

| Setter | Effect |
|---|---|
| `order` | Try these first, in this order. |
| `only` | Restrict routing to these and nothing else. |
| `ignore` | Exclude these. |

## Control fallback

By default OpenRouter falls back to another provider when the first choice
fails. To forbid that:

```rust
let prefs = ProviderPreferences::builder()
    .allow_fallbacks(FallbackPolicy::Deny)
    .build();
```

`FallbackPolicy::Deny` serialises to `false`, `Allow` to `true`. The API's
default when the field is absent is `true`.

## Sort the candidates

```rust
let prefs = ProviderPreferences::builder().sort(ProviderSort::Latency).build();
```

| Variant | Wire | Sorts by |
|---|---|---|
| `Price` | `"price"` | cheapest first |
| `Throughput` | `"throughput"` | highest tokens/second first |
| `Latency` | `"latency"` | lowest time-to-first-token first |
| `Exacto` | `"exacto"` | exact ordering, no fallback reordering |

The object form of `sort` (`{by, partition}`) is not modelled — `Sort` is the
enum above and nothing else.

## Cap what you pay

`MaxPrice` is a partial object of USD-per-million-token caps. Its fields are
public and default to `None`; set only the ones you care about.

```rust
use openrouter_rs::types::Price;

let mut mp = MaxPrice::new();
mp.prompt = Some(Price::new(1.0)?);
mp.completion = Some(Price::new(3.0)?);

let prefs = ProviderPreferences::builder().max_price(mp).build();
```

| Field | Caps |
|---|---|
| `prompt` | prompt-token cost, USD per million tokens |
| `completion` | completion-token cost, USD per million tokens |
| `image` | image-token cost |
| `audio` | audio-token cost |
| `request` | cost **per request**, not per token |

`Price::new` rejects `NaN`, infinities, and negatives.

## Set performance floors and ceilings

```rust
let prefs = ProviderPreferences::builder()
    .preferred_min_throughput(ThroughputFloor::new(50.0)?)   // tokens/second
    .preferred_max_latency(LatencyCeiling::new(2.0)?)        // seconds to first token
    .build();
```

Both reject `NaN`, infinities, and negatives. Note these two types live in
`openrouter_rs::chat`, not `openrouter_rs::types` — the prelude re-exports both.

## Constrain data handling

```rust
let prefs = ProviderPreferences::builder()
    .data_collection(DataCollection::Deny)
    .zdr(ZeroDataRetention::Enabled)
    .build();
```

`DataCollection` serialises to `"allow"` / `"deny"`. `ZeroDataRetention`
serialises to a bare `true` / `false` under the key `zdr`.

## Require parameter support

By default a provider that does not support a parameter you sent may simply
ignore it. To route only to providers that honour every parameter in the
request:

```rust
let prefs = ProviderPreferences::builder()
    .require_parameters(ParameterRequirement::Required)
    .build();
```

Serialises to `true`. Pair it with
[structured outputs](request-structured-outputs.md) or unusual sampling
parameters, where a silently ignored field is worse than a routing failure.

## Restrict quantization

```rust
let prefs = ProviderPreferences::builder()
    .quantizations(vec![Quantization::Fp16, Quantization::Bf16])
    .build();
```

Accepted levels: `Int4`, `Int8`, `Fp4`, `Fp6`, `Fp8`, `Fp16`, `Bf16`, `Fp32`,
`Unknown`. Each serialises to its verbatim wire token (`"int4"`, `"bf16"`, …).

## Fall back to a different model

Provider routing picks *who serves* a model. To fall back to a *different model*
entirely, use the request-level `models` list:

```rust
let req = ChatRequest::builder()
    .model(ModelId::custom("openai/gpt-4o")?)
    .models(vec![
        ModelId::custom("anthropic/claude-3-haiku")?,
        ModelId::custom("openai/gpt-4o-mini")?,
    ])
    .messages(vec![Message::user("hi")])
    .build();
```

## A note on booleans

Every yes/no routing control is a two-variant enum rather than a `bool`
(`FallbackPolicy`, `ParameterRequirement`, `ZeroDataRetention`,
`SchemaStrictness`). Each has an `as_bool()` accessor and serialises to a bare
JSON boolean. The reason is readability at the call site: `.allow_fallbacks(Deny)`
says what it means; `.allow_fallbacks(false)` does not.
