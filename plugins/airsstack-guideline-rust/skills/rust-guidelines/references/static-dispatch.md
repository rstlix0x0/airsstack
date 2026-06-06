# Rust — Prefer Static Dispatch (Avoid `Box<dyn Trait>`)

**Static dispatch via generics is the default**. `Box<dyn Trait>` is a last resort. This rule strengthens and localizes `M-DI-HIERARCHY` and `M-AVOID-WRAPPERS` from the Microsoft guidelines reference.

Cross-links: the Microsoft guidelines reference (`M-DI-HIERARCHY`, `M-AVOID-WRAPPERS`, `M-SIMPLE-ABSTRACTIONS`, `M-SERVICES-CLONE`), the strong-types reference.

## The rule

When you need to call methods on a value through an abstraction, choose in this order:

1. **Concrete type** — if there is only one real implementation, use it directly. Generics for "one day someone might swap this" is speculative.
2. **Generic with trait bound** (`<T: Trait>` / `impl Trait` / `where T: Trait`) — when more than one concrete implementation is genuinely expected (e.g. the production `SqlStorage` and a `MockStorage` for tests). This is the default for *behaviour injection*.
3. **`&dyn Trait` or `&mut dyn Trait`** — short-lived borrowed dispatch. Cheapest dyn form; no heap allocation. Acceptable in narrow internal paths.
4. **`Box<dyn Trait>` / `Arc<dyn Trait>`** — only with a documented justification (see [Justified exceptions](#justified-exceptions) below). Reviews reject unjustified trait objects.

## Why

External sources align on the same recommendation:

- **Effective Rust, Item 12**: *"prefer generics to trait objects."* Trait objects pay two indirections (object → vtable → impl), and they constrain what the trait can declare (object-safety rules forbid generic methods and `Self`-in-arguments).
- **Rust API Guidelines**: `C-GENERIC` (generics give zero-cost abstraction) is preferred over erased dispatch unless the API needs heterogeneous storage.
- **`M-SIMPLE-ABSTRACTIONS`**: nested-generic *signatures* are unreadable, but a single `<S: Trait>` parameter on a `Repository<S>` does not nest; it is one bound at one level. Reach for type aliases (`pub type DefaultRepository = Repository<SqlStorage>;`) to keep call sites short.

For a library that runs inside larger applications (a CLI or service built on it is one consumer), monomorphization pays back: the application binary gets one optimized copy of the hot path; clones of the service handle share an `Arc`-pooled state without going through a vtable on every call.

## `Box<dyn Trait>` vs `Arc<Inner>` — different patterns, do not conflate

This rule targets **trait-object boxing for behaviour injection**. It does NOT prohibit the unrelated and idiomatic `Arc<Inner>` cheap-Clone pattern.

| Pattern                          | Status | Reason                                                                                                  |
| -------------------------------- | :----: | ------------------------------------------------------------------------------------------------------- |
| `Box<dyn Storage>` field         |  AVOID | Dyn dispatch on every call; pick a generic `Repository<S: Storage>` and a type alias for default.       |
| `Arc<Inner>` inside a `Repository` |   OK   | `M-SERVICES-CLONE`: long-lived service handles share state across `Clone` via an internal `Arc<Inner>`. The cloned `Repository` is a refcount bump, not a vtable. Matches the pattern used by `reqwest`, `sqlx::Pool`, and AWS SDK handles. |
| `Box<T>` for owned heap data     | RARELY | Only when boxing slims a large enum variant (`enum E { Small, Huge(Box<HugePayload>) }`), breaks a recursive type, or supports an `async` future stored in a struct. Document the reason inline. |
| `Box<dyn Future>` / `Pin<Box<…>>`| RARELY | Allowed where `async fn` in a trait method forces type erasure (`async-trait` crate expands to this). Treat it as a transient cost of dyn-compat async, not a default. |
| `&dyn Trait` / `&mut dyn Trait`  |   OK   | No heap, cheap dispatch. Use for short-lived borrowed callbacks (e.g. visitor passes).                  |
| `Arc<dyn Trait>` shared sink     | RARELY | Allowed when callers must register heterogeneous implementations *at runtime* (plugin systems, event subscribers). Not a substitute for laziness about generics. |

Rule of thumb: **if the implementations are known at compile time (production impl + mocks + maybe a second backend), generics.** **If the implementations arrive at runtime (loaded plugins, user-supplied callbacks of unknown type), trait objects.**

## Justified exceptions

A `Box<dyn Trait>` or `Arc<dyn Trait>` is acceptable when one or more of these hold, AND a code comment (`// dyn: ...`) records which:

1. **Heterogeneous collection**: `Vec<Box<dyn EventHandler>>` of subscribers of unknown concrete type.
2. **Runtime-loaded backend**: a plugin system where the impl ships in a separate crate the SDK does not import directly.
3. **Object-safe public hook**: an extension point the SDK *exposes* to users so they can plug their own implementation without each crate having to be generic over it. Document with a doc-comment that this is the user extension seam.
4. **Code-size cliff**: monomorphization of a large method body across many type parameters measurably bloats the binary, and benchmarks show dyn dispatch is a net win. Include the measurement in the comment.
5. **`async fn` in trait** until `dyn`-compat for async-in-trait is stable / ergonomic. Use `async-trait` macro and tolerate the `Pin<Box<…>>` it expands to. Prefer native `async fn` in trait + generic dispatch where dyn is not needed.

A comment that just says `// using Box<dyn Trait> for flexibility` does not justify anything. Name the concrete reason.

## How this affects API design

### `Repository<S>` over `Repository { storage: Box<dyn Storage> }`

Default pattern for any service or repository handle:

```rust
pub struct Repository<S = SqlStorage>
where
    S: Storage,
{
    inner: Arc<RepositoryInner<S>>,   // Arc-Inner for cheap Clone, NOT for dyn dispatch
}

pub type DefaultRepository = Repository<SqlStorage>;   // keep call sites short

impl<S: Storage> Clone for Repository<S> {
    fn clone(&self) -> Self { Self { inner: Arc::clone(&self.inner) } }
}
```

- The default type parameter (`<S = SqlStorage>`) means typical callers write `Repository::builder()...build()` and get the `Default` substitution — no generic noise.
- Tests substitute `Repository<MockStorage>` with no production-path indirection cost.
- Public surface stays clean because the only place users see `<S>` is in the type-alias / default substitution.

### Builder methods that take callbacks

For one-shot callbacks (`on_retry`, `on_response`) prefer `Fn`/`FnOnce` generics:

```rust
pub fn on_retry<F: Fn(&RetryEvent) + Send + Sync + 'static>(self, f: F) -> Self
```

Box the closure internally if you must store many of them or store at runtime; do not force callers through `Box<dyn Fn…>` at the call site.

## Things to AVOID

- Writing `Box<dyn Trait>` because "it's simpler" — generic field with a default type parameter is one extra line at the declaration and zero extra at the call site.
- Erasing the type of a long-lived service handle behind `dyn` only to enable `Clone`. Use `Arc<Inner>` instead — concrete inner, refcount on the outside.
- `async-trait` for *internal* traits the library only consumes itself. Native `async fn` in trait + generic dispatch is dyn-free and zero-cost. Reserve `async-trait` for traits exposed to downstream users where dyn-compat matters.
- Nested generics in public signatures (`Foo<Bar<Baz<T>>>`). If a generic parameter is leaking that deep, refactor with a type alias (`M-SIMPLE-ABSTRACTIONS`).

## Definition of Done (rule additions)

In addition to the strict-quality reference DoD:

- `grep -rn 'Box<dyn ' crates/` returns no new hits without a matching `// dyn:` comment naming the justified-exceptions clause.
- `grep -rn 'Arc<dyn ' crates/` same rule.
- Public types do not expose a `dyn Trait` in their signature unless documented as a user extension seam.
- Code review must call out any `Box<dyn …>` introduced by the change.
