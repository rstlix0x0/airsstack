# Type-state builders

Two builders in this crate enforce required fields at compile time:
`ClientBuilder` (requires `api_key`) and `ChatRequestBuilder` (requires `model`
and `messages`). Neither has a runtime "missing field" error, because a missing
field never produces a value to check.

## The problem with the ordinary pattern

The conventional Rust builder collects `Option`s and validates in `build()`:

```rust
pub fn build(self) -> Result<Client, BuildError> {
    let api_key = self.api_key.ok_or(BuildError::MissingApiKey)?;
    // ...
}
```

Every caller now handles an error that is not a runtime condition at all — it is
a typo. The compiler knew, and said nothing.

## What this crate does instead

The builder carries a type parameter for each required field, inhabited by one
of two marker types:

```rust
pub struct ClientBuilder<Key, T> where Key: BuilderApiKeyState, T: HttpTransport

pub struct Missing;
pub struct Present;
```

`build()` is implemented only on the `Present` state:

```rust
impl<T: HttpTransport> ClientBuilder<Missing, T> {
    pub fn api_key(self, key: ApiKey) -> ClientBuilder<Present, T> { /* ... */ }
}

impl<T: HttpTransport> ClientBuilder<Present, T> {
    pub fn build(self) -> Result<Client<T>, BuildError> { /* ... */ }
}
```

Forget the key and the compiler says `no method named 'build' found`. There is no
error variant to define, no error for the caller to handle, and no test needed to
prove the check fires.

`ChatRequestBuilder<M, Ms>` does the same with two parameters, so `build()`
exists only on `ChatRequestBuilder<Present, Present>`. Order does not matter: the
`model` transition is implemented for any `Ms`, and the `messages` transition for
any `M`.

## The states are sealed

```rust
mod sealed { pub trait Sealed {} }
pub trait BuilderApiKeyState: sealed::Sealed {}
```

`Missing` and `Present` are the only inhabitants, and a downstream crate cannot
add a third — the supertrait it would need to implement is not nameable outside
this module. The state machine is closed, which is what makes reasoning about it
finite.

## The field-drop hazard, and the fix

A type-state transition returns a *different type*, so it has to reconstruct the
builder. The naive version enumerates fields:

```rust
ClientBuilder {
    api_key: Some(key),
    http_referer: self.http_referer,
    app_title: self.app_title,
    // forget one here and it silently vanishes
    _key: PhantomData,
}
```

Every new optional field is a chance to drop a value at the transition, and the
symptom — a setting that works only if you call it *after* `.api_key(...)` — is
maddening to debug.

The fix is to put every mutable field in one private non-generic struct:

```rust
struct ClientBuilderFields {
    api_key: Option<ApiKey>,
    http_referer: Option<String>,
    app_title: Option<String>,
    timeout: Option<Duration>,
    base_url: Option<BaseUrl>,
}

pub struct ClientBuilder<Key, T> {
    fields: ClientBuilderFields,   // ← moved whole across the transition
    transport: T,
    _key: PhantomData<Key>,
}
```

The transition moves `fields` as a single value. Adding a field touches the
struct, its constructor, the setter, and `build()` — never the transition code.
The hazard is designed out rather than remembered.

`ChatRequestFields` does the same for the request builder, using
`ChatRequestFields { model: Some(m), ..self.fields }` for the same reason.

Both builders' tests guard this directly: set an optional *before* the required
transition and assert it survives.

## Proving the negative

You cannot write a unit test for "this does not compile" — the test file would
not compile either. The proof lives in `tests/compile_fail/`, driven by
`trybuild`, which compiles each fixture and compares the diagnostic against a
checked-in `.stderr` golden file:

```
tests/compile_fail/client_missing_api_key.rs   + .stderr
tests/compile_fail/chat_missing_model.rs       + .stderr
tests/compile_fail/chat_missing_messages.rs    + .stderr
tests/compile_pass/full_chat_request.rs
```

The `compile_pass` fixture matters as much as the failures: it proves the
positive case still builds, so a change that accidentally makes `build()`
uncallable everywhere does not pass by making all three failures fail harder.

After an intentional signature change, regenerate rather than hand-edit:

```bash
TRYBUILD=overwrite cargo test -p openrouter-rs --test builder_compile
```

## When `build()` still returns `Result`

`ClientBuilder::build()` returns `Result<Client<T>, BuildError>` even though it
cannot fail. The signature keeps the caller writing `?`, which is what makes
`BuildError` extensible without breaking downstream code.

`ChatRequestBuilder::build()` returns a bare `ChatRequest`. Every value it holds
is an already-validated newtype, and the builder holds no cross-field rule, so
there is nothing for a `Result` to carry.

Both contain one `expect` on the required-field `Option`, unreachable by
construction, with the type-state invariant named in the reason string.

## The cost

Type-state is not free. The builder's type changes as you call it, which means
you cannot easily store a partially-built builder in a struct field or return one
from a function without naming its state. For a fluent chain — the overwhelmingly
common usage — this never surfaces. For a builder assembled across several
functions, it does, and a plain builder would be the better choice.

That is exactly the reasoning behind `ProviderPreferencesBuilder` being an
ordinary fluent builder: it has no required fields, so there is nothing for the
type system to prove.
