# Reference — Domain types

Module `crate::types`, plus two chat-local numerics noted below. Every type
validates at construction and exports its rejection reason alongside it. See
[validated domain types](../explanation/validated-domain-types.md) for the why.

## Identifiers and strings

| Type | Rule | Error type | Serde |
|---|---|---|---|
| `ApiKey` | non-empty; every byte `ascii_graphic` (no whitespace, no non-ASCII) | `InvalidApiKey::{Empty, NonPrintable}` | none — never serialised |
| `BaseUrl` | absolute URL; scheme must be `http` or `https` | `InvalidBaseUrl::{Malformed, UnsupportedScheme}` | none |
| `ModelId` | non-empty; no Unicode whitespace | `InvalidModelId::{Empty, Whitespace}` | transparent |
| `FunctionName` | 1–64 bytes; `[A-Za-z0-9_-]` only | `InvalidFunctionName::{Empty, TooLong, InvalidChar}` | transparent |
| `SchemaName` | 1–64 bytes; `[A-Za-z0-9_-]` only | `InvalidSchemaName::{Empty, TooLong, InvalidChar}` | transparent |
| `ProviderSlug` | non-empty; no Unicode whitespace | `InvalidProviderSlug::{Empty, Whitespace}` | transparent |
| `ToolCallId` | non-empty | `InvalidToolCallId::Empty` | transparent |

`FunctionName` and `SchemaName` enforce the identical rule but are distinct
types — the compiler will not let you pass one where the other is expected.

Constructors: `ApiKey::new`, `BaseUrl::parse`, `ModelId::custom`, and `new` for
the rest. `ModelId`, `FunctionName`, `SchemaName`, `ProviderSlug`, and
`ToolCallId` also implement `FromStr` delegating to their constructor, and
`Display` returning the inner string.

### `ApiKey`

Backed by `secrecy::SecretString`. `Debug` prints `ApiKey("***")`.
`expose_secret() -> &str` is the only way out, and is used exactly once — to
build the `Authorization` header.

### `BaseUrl`

Wraps a `url::Url` that stays private, so `url` never appears in the public
surface and a `url` version bump is not a breaking change. `as_str()` and
`Display` expose it as a string.

`http` is permitted so you can target a local proxy or test server. `file`,
`data`, `ftp`, and everything else are rejected.

Path joining (crate-private `join`) follows RFC 3986:

```text
"https://host"         + "chat/completions" → "https://host/chat/completions"
"https://host/api/v1/" + "chat/completions" → "https://host/api/v1/chat/completions"
"https://host/api/v1"  + "chat/completions" → "https://host/api/chat/completions"   ← drops v1
```

**Configure a base URL whose path ends with `/`.** The default,
`https://openrouter.ai/api/v1/`, does; endpoint path constants carry no leading
slash.

### `ModelId`

Routing-hint suffixes are accepted verbatim: `openai/gpt-4o:nitro` (highest
throughput), `deepseek/deepseek-r1:floor` (lowest price). The crate deliberately
freezes no model list — `ModelId::custom` is the single entry point and cannot go
stale. The authoritative catalog is
[the models endpoint](https://openrouter.ai/api/v1/models); see
[models-catalog.md](models-catalog.md).

## Bounded numerics

Chat sampling parameters. All are `f32`-backed unless noted, all
`#[serde(transparent)]`, all `Copy`, all expose `get()`.

| Type | Range | Fallible | Error |
|---|---|---|---|
| `MaxTokens` (`u32`) | non-zero; no upper bound | ✅ | `InvalidMaxTokens` |
| `Temperature` | finite, `0.0..=2.0` | ✅ | `InvalidTemperature` |
| `TopP` | finite, `0.0..=1.0` | ✅ | `InvalidTopP` |
| `TopK` (`u32`) | any value | ❌ | — |
| `Seed` (`u64`) | any value | ❌ | — |
| `FrequencyPenalty` | finite, `-2.0..=2.0` | ✅ | `InvalidFrequencyPenalty` |
| `PresencePenalty` | finite, `-2.0..=2.0` | ✅ | `InvalidPresencePenalty` |
| `RepetitionPenalty` | finite, `0.0..=2.0` (default 1.0) | ✅ | `InvalidRepetitionPenalty` |

`MaxTokens` imposes no ceiling on purpose: per-model output caps shift with each
release and the server is authoritative.

`NaN` and both infinities are rejected everywhere a float is bounded.

## Lists

### `StopSequences`

`Vec<String>`, 1 to 4 entries. Empty is rejected rather than silently ignored;
five or more is rejected with the count.

| Error | Message |
|---|---|
| `InvalidStopSequences::Empty` | `stop sequences must not be empty` |
| `InvalidStopSequences::TooMany(n)` | `stop sequences must not exceed 4 (got n)` |

`get() -> &[String]`, `into_inner() -> Vec<String>`.
`#[serde(transparent)]` — serialises as a bare JSON array.

## Prices

| Type | Backing | Direction | Rule |
|---|---|---|---|
| `Price` | `f64` | request (routing caps) | finite, `>= 0` |
| `PricePerToken` | `String` | response (catalog) | parses as a finite `f64 >= 0`; original string preserved |

`Price::new` returns `Result<Self, InvalidPrice>`; `get() -> f64`.
`PricePerToken` is documented under
[models-catalog.md](models-catalog.md#pricepertoken). Keeping them separate is
deliberate — one is a limit you send, the other is data you receive, and the
string form preserves precision `f64` would lose.

## Chat-local numerics

Two validated numerics live in `chat::provider` rather than `crate::types`,
because they only exist inside `ProviderPreferences`:

| Type | Unit | Rule |
|---|---|---|
| `ThroughputFloor` (`f64`) | tokens per second | finite, `>= 0` |
| `LatencyCeiling` (`f64`) | seconds | finite, `>= 0` |

Both are re-exported from the crate root and the prelude.

## What these errors are not

None of the `Invalid*` types is a variant of `openrouter_rs::error::Error`. They
implement `std::error::Error` (via `thiserror`) but do not convert into `Error`
with `?`. Handle them where you construct the value. See
[errors.md](errors.md#what-error-does-not-absorb).

## Prelude coverage

The prelude re-exports the newtypes themselves, not their `Invalid*` errors, and
not `BaseUrl`:

```
ApiKey, FrequencyPenalty, FunctionName, MaxTokens, ModelId, PresencePenalty,
Price, PricePerToken, ProviderSlug, RepetitionPenalty, SchemaName, Seed,
StopSequences, Temperature, ToolCallId, TopK, TopP
```

Everything else is reachable as `openrouter_rs::types::<Name>`.
