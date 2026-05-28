---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---

# Rust — Strong Types (No Primitive Obsession)

In this repo, **every public parameter and field carries meaning in its type**. A bare `String`, `u32`, `bool`, or `&str` on a public API is presumed wrong unless the value is a true universal scalar with no domain semantics. This rule strengthens `M-STRONG-TYPES`, `M-PUBLIC-DEBUG`, `M-PUBLIC-DISPLAY`, and `M-ESCAPE-HATCHES` from [[rust-microsoft-guidelines]].

Cross-links: [[rust-microsoft-guidelines]] (`M-STRONG-TYPES`, `M-ERRORS-CANONICAL-STRUCTS`), [[rust-static-dispatch]], [[rust-strict-quality]].

External anchors:

- Rust API Guidelines: **`C-NEWTYPE`** ("Newtypes provide static distinctions"), **`C-CUSTOM-TYPE`** ("Arguments convey meaning through types"), **`C-VALIDATED`** (constructors enforce invariants), **`C-BUILDER`** (complex initialization).
- Cliffle, *The Typestate Pattern in Rust* — state-machine APIs that make invalid transitions a compile error.
- Alexis King's *Parse, Don't Validate* — once parsed into a strong type, downstream code never re-checks the invariant.

## The bar

Public APIs in `airsstack` crates MUST:

1. **Use a newtype for any string, integer, or boolean that has meaning beyond "a string", "a number", "a flag".**
2. **Validate at construction**, not at point-of-use. `TryFrom<&str>` / `parse` returning `Result<Self, Self::Error>` is the canonical entry point.
3. **Encode mutually exclusive or progressive states in the type system** — type-state pattern — rather than runtime checks (`if self.is_built { ... }`).
4. **Never accept `bool` for two-state semantic flags**. Use a two-variant enum named for the decision.

## Newtype pattern

### When you MUST newtype

Any of the following on a public boundary:

- **Identifiers**: `ApiKey`, `OrganizationId`, `RequestId`, `MessageId`, `ToolUseId`, `ModelId`. They're not interchangeable strings; the compiler should refuse swaps.
- **Tokens / opaque secrets**: wrap in `SecretString` (`secrecy` crate) inside the newtype so `Debug` and accidental `format!` do not leak material.
- **Bounded numerics**: `MaxTokens(u32)`, `Temperature(f32)`, `TopP(f32)`, `TopK(u32)`, `BackoffMillis(u64)`. The constructor enforces ranges (`0..=4_096` for `MaxTokens`, `0.0..=1.0` for `Temperature` etc.) and the rest of the codebase trusts the invariant.
- **Domain strings with a syntax**: `ModelId`, `AnthropicVersion`, `BetaHeader`, `StopSequence`, `SystemPromptSegment`. Even when the inner representation is `String`, the newtype documents the contract.
- **Units**: `Duration` from `std::time` rather than `u64` milliseconds. Never `u64 = 5000` as a "timeout".

### When NOT to newtype

- The value is a **generic scalar with no domain meaning** at this layer: a buffer length passed to `Read::read`, a `usize` index inside a private hot loop.
- The newtype would have **zero invariants and zero distinguishing methods** — e.g. an internal helper struct field used in exactly one place. Wait for a second use site before extracting.

### Canonical newtype shape

```rust
/// A Claude model identifier accepted by the Messages API.
///
/// Use one of the associated constants for known models. Use [`ModelId::custom`]
/// for IDs the SDK does not yet know about.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ModelId(String);

impl ModelId {
    pub const CLAUDE_OPUS_4_7:    Self = Self(/* compile-time const ctor */);
    pub const CLAUDE_SONNET_4_6:  Self = Self(/* ... */);

    /// Construct a `ModelId` for an arbitrary string.
    ///
    /// # Errors
    /// Returns [`InvalidModelId`] if `s` is empty or contains whitespace.
    pub fn custom(s: impl Into<String>) -> Result<Self, InvalidModelId> { /* validate */ }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for ModelId { /* prints inner */ }
impl std::str::FromStr for ModelId {
    type Err = InvalidModelId;
    fn from_str(s: &str) -> Result<Self, Self::Err> { Self::custom(s) }
}
```

Notes:

- `#[serde(transparent)]` so the wire format is unchanged.
- `Debug` derived; `Display` hand-written or derived via `derive_more` per `M-PUBLIC-DISPLAY`.
- Validation in the constructor (`custom`). After construction, downstream code never re-checks.
- `as_str()` returns the inner view; do NOT expose the inner field as `pub`.
- For secrets: wrap inner as `SecretString`, `Debug` masks ("`ApiKey(****)`"), no `Display`.

### Validated numeric newtype

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaxTokens(u32);

impl MaxTokens {
    pub const fn new(n: u32) -> Result<Self, InvalidMaxTokens> {
        if n == 0 || n > 32_000 { return Err(InvalidMaxTokens(n)); }
        Ok(Self(n))
    }
    pub const fn get(self) -> u32 { self.0 }
}
```

Const constructors give compile-time validation for literals (`MaxTokens::new(1024).expect("literal valid")` → checkable in `const` context). The `# Errors` doc section is required per `M-CANONICAL-DOCS`.

### Don't reach for the `nutype` macro by default

Prefer hand-written newtypes for SDK public types. Macro-generated newtypes hide the API surface from readers, and SDK callers benefit from explicit `# Errors` / `# Examples` rustdoc. Use `nutype`/`derive_more` for *internal* boilerplate-heavy newtypes if at all.

## No `bool` parameters

`bool` is `M-STRONG-TYPES`-banned at public boundaries when it expresses a decision:

```rust
// BAD: caller writes `client.send(msg, true)` — true what?
pub fn send(&self, msg: Message, retry: bool) -> Result<…>

// GOOD:
#[derive(Clone, Copy, Debug)]
pub enum RetryPolicy { Disabled, ExponentialBackoff }
pub fn send(&self, msg: Message, retry: RetryPolicy) -> Result<…>
```

Two-state enums cost the same as `bool`, document the call site, and leave room to grow (a third variant like `LinearBackoff { interval: Duration }` is a non-breaking change for callers who already use a named enum, but a breaking change if you tried to expand a `bool`).

Exceptions: `bool` is fine when the parameter is truly boolean ("does this match?"), e.g. a predicate result, an `Option::is_some`-style query, an internal helper. Public API `fn foo(x: bool)` where `bool` answers "yes / no to X" needs a `IsX` enum.

## Type-state pattern

Use type-state when:

- An API requires **a specific order** of operations (configure → connect → authenticate → use).
- Some fields are **required** before a constructor can succeed and others are optional. The builder should refuse to compile `build()` until the required fields are set.
- The valid operations on a value **change as the value progresses** through its lifecycle (open → in-progress → committed).

Implementation outline (Cliffle's recommended shape):

```rust
mod sealed {
    pub trait Sealed {}
}

pub trait BuilderState: sealed::Sealed {}

pub struct Missing;
pub struct Present;
impl sealed::Sealed for Missing {}
impl sealed::Sealed for Present {}
impl BuilderState for Missing {}
impl BuilderState for Present {}

pub struct ClientBuilder<ApiKey = Missing>
where
    ApiKey: BuilderState,
{
    api_key: Option<crate::ApiKey>,
    timeout: Option<Duration>,
    _marker: PhantomData<ApiKey>,
}

impl ClientBuilder<Missing> {
    pub fn new() -> Self { /* ... */ }

    pub fn api_key(self, key: crate::ApiKey) -> ClientBuilder<Present> {
        ClientBuilder {
            api_key: Some(key),
            timeout: self.timeout,
            _marker: PhantomData,
        }
    }
}

impl<S: BuilderState> ClientBuilder<S> {
    pub fn timeout(mut self, t: Duration) -> Self { self.timeout = Some(t); self }
}

// `build` only exists once api_key is `Present`.
impl ClientBuilder<Present> {
    pub fn build(self) -> Result<Client, BuildError> { /* ... */ }
}
```

Compile-time effect:

- `ClientBuilder::new().build()` → **compile error**: no `build` method on `ClientBuilder<Missing>`.
- `ClientBuilder::new().api_key(k).build()` → compiles.
- The `Sealed` trait closes the state set so downstream crates cannot invent new `BuilderState` impls.

### When type-state is overkill

Cliffle's caution applies: do not type-state a two-state on/off lifecycle that runtime checks handle trivially. Save it for:

- Required-vs-optional builder fields (the canonical use).
- HTTP-response-style ordered phases (status → headers → body).
- Long-lived handles whose available operations change after a state transition (e.g. an authenticated session vs. an unauthenticated one).

For trivial cases, a `Result<Built, BuildError>` returned from `build()` is simpler.

## Validated parse, then trust

Per *Parse, Don't Validate*: once a value is wrapped in its newtype, downstream code does not re-validate. The newtype IS the proof.

```rust
// BAD — function takes the unrestricted primitive and re-checks every call.
fn send_messages(model: String, max_tokens: u32) -> Result<…> {
    if model.is_empty() { return Err(...); }
    if max_tokens == 0 { return Err(...); }
    /* ... */
}

// GOOD — types carry the invariant once.
fn send_messages(model: ModelId, max_tokens: MaxTokens) -> Result<…> {
    /* no validation needed — the types prove it */
}
```

This compounds with `M-PANIC-IS-STOP` (panics signal *bugs*, not user errors): downstream `unwrap()` on a type whose invariants are enforced at construction is *not* a `unwrap` to remove — it is an assertion that the type system is doing its job. Use `.expect("invariant: <type> guarantees <property>")` to document why panicking is sound.

## Errors from validating constructors

Per `M-ERRORS-CANONICAL-STRUCTS`: validation errors are structs (or enum-of-structs), not `String`. Each newtype has its own error type (`InvalidModelId`, `InvalidMaxTokens`) implementing `std::error::Error` via `thiserror`. The constructor returns `Result<Self, MyInvalidErrorType>`. Errors are not wrapped in `Box<dyn Error>` at the construction boundary (per [[rust-static-dispatch]]).

## Things to AVOID

- Public function signatures with bare `&str` or `String` for domain-meaningful values. Always wrap.
- Validation happening at the API call site rather than the type constructor. Validate once, trust everywhere.
- `pub struct Foo(pub String)` — exposing the inner field defeats the newtype. Either expose `as_str(&self) -> &str` and `into_inner(self) -> String`, or document why the field is public.
- `bool` parameters that answer a domain question. Use a named two-variant enum.
- Type-state machinery applied to a problem that does not need it. Two pieces of evidence: (a) there is a clear ordering or required-fields constraint, (b) the runtime check version would be ugly or unsafe. Both must hold.
- Re-validating inside library functions what the constructor has already validated. Treat the type as the proof and `.expect("invariant: …")` if you must unwrap.

## Definition of Done (rule additions)

In addition to [[rust-strict-quality]] DoD:

- Every new public function parameter or struct field MUST be a domain type, not a primitive, unless it is a true generic scalar with no domain semantics. Reviewer flags violations.
- Every new newtype has: `Debug`, fallible construction (`TryFrom` or named ctor), `# Errors` doc, at least one doctest demonstrating the happy path and one error path.
- Builders with required fields use the type-state pattern; `build()` must NOT be callable until the type-state proves required fields are present.
- `bool` does not appear in any new public function parameter where it answers a semantic question.
- Doctests in `# Examples` demonstrate constructing through the validated entry point, never through a raw `Foo("...".into())` literal.
