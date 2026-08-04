# Validated domain types

There are no bare `String`s or bare `f32`s on this crate's request surface.
Every value that carries meaning is a newtype that validated itself when it was
constructed. The full catalogue is in
[reference/domain-types.md](../reference/domain-types.md); this page is about
why.

## Parse, don't validate

The distinction is where the check lives relative to the type:

```rust
// validate: the invariant is a fact about the code path, not the value
fn send(model: &str) -> Result<Completion, Error> {
    if model.is_empty() { return Err(...); }
    // ... and the next function that takes `model: &str` checks again, or forgets
}

// parse: the invariant is a fact about the value
fn send(model: ModelId) -> Result<Completion, Error> {
    // ModelId cannot be empty. There is no code path where it is.
}
```

Once `ModelId::custom` has returned `Ok`, every downstream function that accepts a
`ModelId` knows it is non-empty and whitespace-free, and none of them re-checks.
The check happens once, at the boundary, and the result is encoded in the type.

This is why `ChatRequestBuilder::build()` is infallible. By the time you call it,
every value it holds has already proven itself. There is nothing left to reject.

## Failure reasons travel with the type

Each newtype exports its rejection reason next to it — `ApiKey` /
`InvalidApiKey`, `Temperature` / `InvalidTemperature`, `StopSequences` /
`InvalidStopSequences`. The error names the specific violation:

```rust
InvalidStopSequences::TooMany(5)     // "stop sequences must not exceed 4 (got 5)"
InvalidBaseUrl::UnsupportedScheme("ftp".into())
InvalidFunctionName::InvalidChar
```

Compare with a single `BuildError::InvalidConfig("bad value")`, which tells the
caller nothing about which field or which rule.

## These are not SDK errors

`InvalidModelId` is deliberately **not** a variant of
`openrouter_rs::error::Error`, and `?` will not convert it. That is a statement
about kind, not an oversight: `Error` is the failure domain of *talking to the
API*, and a malformed model id never gets that far. Mixing them would mean every
`match` on an API response also had to consider a category of failure that
happened before the request existed.

The practical consequence is that you handle newtype errors at the construction
site. See
[reference/errors.md](../reference/errors.md#what-error-does-not-absorb).

## Distinct types for identical rules

`FunctionName` and `SchemaName` enforce the same rule — `[A-Za-z0-9_-]`, 1 to 64
characters — and are still separate types. Collapsing them into one
`ShortIdentifier` would let a schema name be passed where a function name belongs
and compile cleanly. The duplication buys a compile error at that call site.

The same reasoning separates `Price` (`f64`, request-side routing caps) from
`PricePerToken` (decimal string, response-side catalog data). They are numerically
similar and semantically opposite — one is a limit you impose, the other is data
you receive.

## Where representation follows the wire

`PricePerToken` keeps its decimal string. `"0.0000003"` does not survive a
round trip through `f64`, and the catalog is data you may want to display or
store exactly as the server stated it. `as_str()` gives you the wire value;
`to_f64()` gives you arithmetic and the rounding that comes with it. `PartialEq`
compares the string, so `"0.5" != "0.50"` — surprising until you remember the
type's job is fidelity, not numerics.

`FunctionCall::arguments` is the same decision one level up: the model's tool
arguments stay a raw JSON string rather than a parsed `serde_json::Value`,
because re-serialising a payload you did not author can change it.

## Two-variant enums instead of `bool`

Semantic flags are enums, not booleans:

| Type | Instead of |
|---|---|
| `FallbackPolicy::{Allow, Deny}` | `allow_fallbacks: bool` |
| `ParameterRequirement::{Required, Optional}` | `require_parameters: bool` |
| `ZeroDataRetention::{Enabled, Disabled}` | `zdr: bool` |
| `SchemaStrictness::{Strict, Lenient}` | `strict: bool` |
| `CacheMode::{Enabled, Disabled}` | `cache: bool` |
| `CacheClear::{Clear, Keep}` | `clear: bool` |

Each serialises to a bare JSON boolean via `as_bool()`, so the wire format is
unchanged. What changes is the call site:

```rust
.allow_fallbacks(FallbackPolicy::Deny)   // reads as intent
.allow_fallbacks(false)                  // reads as: false what?
```

The gain compounds when two flags sit next to each other. `f(true, false)` is a
puzzle; `f(Strict, Keep)` is a sentence.

The exception proves the rule: `FunctionDef::strict` is a plain `Option<bool>`,
because it is a pass-through field on a struct the caller fills in literally,
not a parameter in a fluent chain.

## Where validation deliberately stops

Not every constraint is worth encoding.

- **`MaxTokens` has no upper bound.** Per-model output caps change with every
  release; the server is authoritative, and a stale client-side ceiling would
  reject valid requests.
- **`ModelId` freezes no model list.** `ModelId::custom` is the only constructor,
  which is exactly why it never goes stale — routing suffixes like
  `openai/gpt-4o:nitro` pass through untouched.
- **`ProviderSlug` has an open charset.** Only non-empty and whitespace-free.
  Provider names are the server's vocabulary, not this crate's.
- **`TopK` and `Seed` are infallible.** Every `u32` and every `u64` is meaningful;
  a `Result` would be noise.

The rule of thumb: encode invariants the *format* guarantees, not policies the
*server* owns.

## The `secrecy` boundary

`ApiKey` wraps a `secrecy::SecretString`. `Debug` prints `ApiKey("***")`, so a
key cannot leak through a logged struct — and `Client`, `Auth`, and `Config` all
inherit that protection by containing it rather than a `String`.

`expose_secret()` is the only way out, and outside its own definition it is
called in exactly two places — once per endpoint, building the `Authorization`
header. Grepping for that method name is a complete audit of where the
credential is read.
