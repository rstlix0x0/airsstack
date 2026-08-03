# Rust — Microsoft Pragmatic Guidelines

Apply the [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/) when writing or reviewing Rust. Spirit over letter: understand the motivation before deviating. The full text and per-guideline rationale lives at the linked site — fetch it when a rule is ambiguous in context.

Each item is identified by its upstream code (`M-*`). Use these codes verbatim in commit messages, review comments, and PR descriptions when a change is motivated by a specific guideline (e.g. `"refactor(api): split crate per M-SMALLER-CRATES"`).

Where a `→` points at another reference, that file is the authority and states the rule in full; the line here exists so the code stays citable.

## Universal — apply everywhere

- **M-UPSTREAM-GUIDELINES** — Follow the upstream [API Guidelines](https://rust-lang.github.io/api-guidelines/), [Style Guide](https://doc.rust-lang.org/nightly/style-guide/), and [Reference](https://doc.rust-lang.org/reference/) before inventing local conventions.
- **M-STATIC-VERIFICATION** — Static checks block the PR. → the Definition of Done in `SKILL.md` (the canonical command set) and `references/strict-quality.md` (the policy).
- **M-LINT-OVERRIDE-EXPECT** — `#[expect(lint, reason = "...")]` over `#[allow]`. → `references/strict-quality.md`
- **M-PUBLIC-DEBUG** — Every public type implements `Debug`. Derive it; hand-roll only to mask secrets. No gaps.
- **M-PUBLIC-DISPLAY** — Types for human consumption implement `Display`, never as a `Debug` substitute. → `references/strong-types.md`
- **M-SMALLER-CRATES** — Prefer multiple focused crates over one mega-crate. → `references/workspace.md`
- **M-CONCISE-NAMES** — Ban weasel words: `Manager`, `Service`, `Helper`, `Util`, `Handler`, `Factory`, `Processor`, `Wrapper`. → `references/modularity.md`
- **M-REGULAR-FN** — `Type::foo` associated functions are for constructors/conversions only. Computation → free functions or `&self` methods.
- **M-PANIC-IS-STOP** — Panic means "program is broken, abort". Never panic as recoverable control flow, never `catch_unwind` to fake exceptions.
- **M-PANIC-ON-BUG** — Programmer bugs panic (invariant violations, impossible variants, index-out-of-bounds in code you control). `Result` is only for what the caller can recover from.
- **M-DOCUMENTED-MAGIC** — Every magic constant gets a comment: origin, why this value, what depends on it. No bare `Duration::from_millis(347)`.
- **M-LOG-STRUCTURED** — `tracing` with named fields: `tracing::info!(user_id = %id, "user logged in")`. No string-concat messages.

## Libraries

### Interoperability

- **M-TYPES-SEND** — Public types are `Send` by default; async runtimes need it. Justify `!Send` in the docs.
- **M-ESCAPE-HATCHES** — Expose raw handles (FDs, sockets, OS objects) via `As*Fd`, `As*Handle`, `into_raw`/`from_raw`. Users eventually need syscalls.
- **M-DONT-LEAK-TYPES** — Never re-export an external crate's types unless that crate is part of your contract — otherwise every dep bump is a breaking change. Newtype-wrap or convert.

### UX

- **M-SIMPLE-ABSTRACTIONS** — No nested generics in public signatures. → `references/static-dispatch.md`
- **M-AVOID-WRAPPERS** — Keep `Arc`/`Rc`/`Box`/`Mutex`/`RefCell` out of public APIs. → `references/static-dispatch.md`
- **M-DI-HIERARCHY** — Concrete types → generics → `dyn Trait`, in that order. → `references/static-dispatch.md`
- **M-ERRORS-CANONICAL-STRUCTS** — Error types are structs (or enums of structs), never `String`. → `references/strong-types.md`
- **M-INIT-BUILDER** — More than ~4 params, or many optionals, means a builder. Not `new(a, b, c, d, e, f, g)`.
- **M-INIT-CASCADED** — Parent builds child by passing context; the child never reaches up. One-way data flow.
- **M-SERVICES-CLONE** — Long-lived service handles are `Clone`, typically via an internal `Arc`. → `references/static-dispatch.md`
- **M-IMPL-ASREF** — Accept `impl AsRef<str>` / `AsRef<Path>` over `&str`/`&Path` where it is cheap, so callers pass `String` or `PathBuf` directly.
- **M-IMPL-RANGEBOUNDS** — Range-taking APIs accept `impl RangeBounds<T>`: `0..10`, `..=5`, `3..`.
- **M-IMPL-IO** — Sans-IO: accept `impl Read` / `impl Write` / `impl AsyncRead`, not concrete `File`/`TcpStream`.
- **M-ESSENTIAL-FN-INHERENT** — Core operations are inherent methods. No `use FooTrait` to call the obvious method.

### Resilience

- **M-MOCKABLE-SYSCALLS** — Wrap syscalls and I/O behind a trait or fn-pointer so tests substitute fakes. No `std::fs` buried in business logic.
- **M-TEST-UTIL** — Upstream gates downstream-visible test helpers behind a `test-util` feature. **This workspace deviates:** it declares no Cargo features, so `mockall` doubles live in dev-only `test_support` modules (`crates/clauders/src/test_support.rs`, `crates/openrouter-rs/src/test_support.rs`). Apply the intent — lean production builds — not the mechanism.
- **M-STRONG-TYPES** — No primitive obsession: `UserId(u64)` not `u64`. → `references/strong-types.md`
- **M-NO-GLOB-REEXPORTS** — No `pub use foo::*` in libraries; explicit re-exports only. → `references/mod-rs-export-only.md`
- **M-AVOID-STATICS** — No `static mut`, no hidden global state; pass dependencies explicitly. If unavoidable, `OnceLock`/`LazyLock` with a documented lifetime.

### Building

- **M-OOBE** — Library compiles and its basic example runs with zero config after `cargo add`. No env vars or external services required for the default-build `cargo test`.
- **M-SYS-CRATES** — `*-sys` crates build from their declared system library alone. No surprise runtime deps.
- **M-FEATURES-ADDITIVE** — Features are purely additive: enabling one never removes an API or changes unrelated behavior. No mutually-exclusive features. (Moot while the workspace is featureless; kept for the day one is added.)

## Applications (binaries, not libraries)

- **M-MIMALLOC-APP** — Binaries set `mimalloc` (or `jemalloc`) as global allocator unless profiling says otherwise. Large multi-threaded throughput win.
- **M-APP-ERROR** — Apps may use `anyhow`/`eyre` for top-level plumbing. Libraries must not — see `M-ERRORS-CANONICAL-STRUCTS`.

## FFI

- **M-ISOLATE-DLL-STATE** — In cdylibs loaded into other processes, isolate global state per library load. Assume no singleton allocator, runtime, or TLS.

## Safety

- **M-UNSAFE** — Every `unsafe` carries a `// SAFETY: ...` comment naming the invariants upheld. Use it only where measurably necessary: FFI, a proven hot path, a sound abstraction over raw primitives.
- **M-UNSAFE-IMPLIES-UB** — Every `unsafe` block is a potential UB site: verify each precondition from the callee's `# Safety` doc at the call site.
- **M-UNSOUND** — A safe public API must not be soundness-breakable from safe code. If safe input can trigger UB the API is broken — fix it, do not document around it.

## Performance

- **M-THROUGHPUT** — Throughput first; no empty cycles. No polling sleeps, `yield_now` loops, or spin-without-backoff.
- **M-HOTPATH** — Find the hot path early, profile it (`cargo flamegraph`, `perf`, `samply`, `tokio-console`), then optimize. Never pre-optimize cold code.
- **M-YIELD-POINTS** — Long sync work in an async context yields (`tokio::task::yield_now`) or moves to `spawn_blocking`. Never starve the runtime.

## Documentation

- **M-FIRST-DOC-SENTENCE** — Doc-comment first line: one sentence, ~15 words, no line break. → `references/doc-comment-discipline.md`
- **M-MODULE-DOCS** — Every `mod` and crate root has `//!` docs. The required four-question block is in → `references/mod-rs-export-only.md`
- **M-CANONICAL-DOCS** — Doc sections in order when relevant: `# Examples`, `# Errors`, `# Panics`, `# Safety`. → `references/doc-comment-discipline.md`
- **M-DOC-INLINE** — `pub use` re-exports get `#[doc(inline)]` when the item is conceptually part of this module's surface. → `references/mod-rs-export-only.md`

## AI

- **M-DESIGN-FOR-AI** — APIs friendly to humans are friendly to agents. Specifically:
  1. Idiomatic — follow Rust API Guidelines so AI matches familiar patterns.
  2. Documented — every module and public item has docs; assume reader has solid but not expert Rust.
  3. Examples — every non-trivial public item has a runnable doctest.
  4. Strong types — no primitive obsession; compiler steers the agent.
  5. Testable — design for fast unit-test iteration loops.
  6. Coverage — high coverage of observable behavior so agents can refactor safely.

Token suppression compounds with this: tight types and high doctest coverage let smaller/cheaper models complete tasks accurately, which is the point of a well-designed AI stack.
