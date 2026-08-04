# Reference — Provider routing

Module `chat::provider`. Everything here serialises under the request's
`provider` key. Set it with `ChatRequestBuilder::provider(prefs)`.

## `ProviderPreferences`

Fields are crate-private; build with `ProviderPreferences::builder()`. Every
field is `Option` and skipped when unset, so empty preferences serialise to `{}`.

| Wire key | Type | Builder setter |
|---|---|---|
| `order` | `Vec<ProviderSlug>` | `order` |
| `only` | `Vec<ProviderSlug>` | `only` |
| `ignore` | `Vec<ProviderSlug>` | `ignore` |
| `allow_fallbacks` | `FallbackPolicy` | `allow_fallbacks` |
| `require_parameters` | `ParameterRequirement` | `require_parameters` |
| `zdr` | `ZeroDataRetention` | `zdr` |
| `data_collection` | `DataCollection` | `data_collection` |
| `quantizations` | `Vec<Quantization>` | `quantizations` |
| `sort` | `ProviderSort` | `sort` |
| `max_price` | `MaxPrice` | `max_price` |
| `preferred_min_throughput` | `ThroughputFloor` | `preferred_min_throughput` |
| `preferred_max_latency` | `LatencyCeiling` | `preferred_max_latency` |

## `ProviderPreferencesBuilder`

A plain fluent builder, **not** a type-state one — nothing is required, so
`build()` is always callable and infallible. Derives `Clone`, `Debug`, `Default`.

Setters taking `Copy` values (`allow_fallbacks`, `require_parameters`, `zdr`,
`data_collection`, `sort`, `max_price`, `preferred_min_throughput`,
`preferred_max_latency`) are `const`; the `Vec`-taking ones are not.

## Boolean-flag enums

Each serialises to a bare JSON boolean and carries an `as_bool()` accessor.

| Type | `true` | `false` | API default when omitted |
|---|---|---|---|
| `FallbackPolicy` | `Allow` | `Deny` | `true` |
| `ParameterRequirement` | `Required` | `Optional` | `false` |
| `ZeroDataRetention` | `Enabled` | `Disabled` | — |

## String-enum controls

### `DataCollection`

`Allow` → `"allow"`, `Deny` → `"deny"`.

### `ProviderSort`

| Variant | Wire | Orders by |
|---|---|---|
| `Price` | `"price"` | lowest cost |
| `Throughput` | `"throughput"` | highest tokens/second |
| `Latency` | `"latency"` | lowest time-to-first-token |
| `Exacto` | `"exacto"` | exact ordering, no fallback reordering |

The object form (`{by, partition}`) is not supported in this release.

### `Quantization`

Verbatim wire tokens: `Int4` → `"int4"`, `Int8` → `"int8"`, `Fp4` → `"fp4"`,
`Fp6` → `"fp6"`, `Fp8` → `"fp8"`, `Fp16` → `"fp16"`, `Bf16` → `"bf16"`,
`Fp32` → `"fp32"`, `Unknown` → `"unknown"`.

## `MaxPrice`

Public fields, all `Option<Price>`, all skipped when `None`. `MaxPrice::new()`
is the all-`None` constructor (equivalent to `Default`); an all-`None` value
serialises to `{}`.

| Field | Caps |
|---|---|
| `prompt` | prompt-token cost, USD per million tokens |
| `completion` | completion-token cost, USD per million tokens |
| `image` | image-token cost |
| `audio` | audio-token cost |
| `request` | **per-request** cost in USD, not per token |

## Chat-local scalar newtypes

These two live in `chat::provider`, not in `crate::types`. Both are
`#[serde(transparent)]` `f64` wrappers serialising as JSON numbers, and both
reject `NaN`, infinities, and negatives at construction.

| Type | Unit | Constructor | Error |
|---|---|---|---|
| `ThroughputFloor` | tokens per second | `new(f64) -> Result<Self, InvalidThroughputFloor>` | `InvalidThroughputFloor::Invalid` |
| `LatencyCeiling` | seconds (time to first token) | `new(f64) -> Result<Self, InvalidLatencyCeiling>` | `InvalidLatencyCeiling::Invalid` |

Both expose `get() -> f64` and are `Copy`. Zero is accepted.

## `ProviderSlug`

Lives in `crate::types`. Non-empty, no whitespace — the charset is deliberately
open because provider names include hyphens, digits, and lowercase letters.
`#[serde(transparent)]`, so a `Vec<ProviderSlug>` serialises as a JSON array of
strings.

## Model-level fallback

Provider routing selects *who serves* a model. The request-level `models` list
(`ChatRequestBuilder::models(Vec<ModelId>)`) is a different mechanism: a chain of
alternative *models* to try if the primary is unavailable. It serialises under
the top-level `models` key, not under `provider`.

## Export locations

Everything in this page is re-exported from the crate root, the `chat` module,
and the prelude, with two exceptions: `InvalidThroughputFloor` and
`InvalidLatencyCeiling` are exported from `chat` only.
