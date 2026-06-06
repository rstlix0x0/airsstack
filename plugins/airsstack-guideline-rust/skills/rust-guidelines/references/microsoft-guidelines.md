# Rust — Microsoft Pragmatic Guidelines

Apply the [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/) when writing or reviewing Rust. Spirit over letter: understand the motivation before deviating. The full text and per-guideline rationale lives at the linked site — fetch it when a rule is ambiguous in context.

Each item below is identified by its upstream code (`M-*`). Use these codes verbatim in commit messages, review comments, and PR descriptions when a change is motivated by a specific guideline (e.g. `"refactor(api): split crate per M-SMALLER-CRATES"`).

## Universal — apply everywhere

- **M-UPSTREAM-GUIDELINES** — Follow the upstream [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), [Style Guide](https://doc.rust-lang.org/nightly/style-guide/), and [Reference](https://doc.rust-lang.org/reference/) before inventing local conventions.
- **M-STATIC-VERIFICATION** — Run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo check`, and where applicable `cargo audit`, `cargo hack`, `cargo udeps`, `miri`. Block PRs on these.
- **M-LINT-OVERRIDE-EXPECT** — Prefer `#[expect(lint_name, reason = "...")]` over `#[allow(...)]`. `#[expect]` errors if the lint stops firing, preventing stale suppressions. Always include the `reason` attribute.
- **M-PUBLIC-DEBUG** — Every public type implements `Debug`. Derive when possible; hand-roll when secrets must be masked. No silent `Debug` gaps in public APIs.
- **M-PUBLIC-DISPLAY** — Types intended for human consumption (errors, string newtypes, IDs, units) implement `Display`. Do NOT implement `Display` as a debug substitute.
- **M-SMALLER-CRATES** — Prefer multiple focused crates over one mega-crate. Faster incremental builds, better parallel codegen, cleaner public surface. Split when a submodule could plausibly stand alone.
- **M-CONCISE-NAMES** — Ban weasel words: `Manager`, `Service`, `Helper`, `Util`, `Handler`, `Factory`, `Processor`, `Wrapper`. Pick a name that says what the type *is*.
- **M-REGULAR-FN** — Associated functions (`Type::foo`) are for constructors/conversions. General-purpose computation → free functions or `&self` methods.
- **M-PANIC-IS-STOP** — Panic = "program is broken, abort". Never use panics as recoverable control flow. Never `catch_unwind` to simulate exceptions in app logic.
- **M-PANIC-ON-BUG** — Programmer bugs (invariant violations, impossible enum variants, index-out-of-bounds in code you control) panic. Return `Result` only for conditions the caller can meaningfully recover from.
- **M-DOCUMENTED-MAGIC** — Every magic constant gets a comment: where it came from, why this value, what depends on it. No bare `Duration::from_millis(347)`.
- **M-LOG-STRUCTURED** — Use structured logging (`tracing` crate) with message templates and named fields: `tracing::info!(user_id = %id, "user logged in")`. No string-concat log messages.

## Libraries

### Interoperability

- **M-TYPES-SEND** — Public types are `Send` by default. Justify `!Send` in docs when unavoidable. Async runtimes and parallelism need it.
- **M-ESCAPE-HATCHES** — Expose raw/native handles (FDs, sockets, OS objects) via `As*Fd`, `As*Handle`, `into_raw`/`from_raw`. Users will need to drop to syscalls eventually.
- **M-DONT-LEAK-TYPES** — Do not re-export types from external crates in your public API unless that crate is part of your contract. Newtype-wrap or convert; otherwise every dep bump is a breaking change.

### UX

- **M-SIMPLE-ABSTRACTIONS** — Avoid nested generics in public signatures. `Foo<Bar<Baz<T>>>` is unreadable. Flatten with type aliases or concrete types.
- **M-AVOID-WRAPPERS** — Don't put `Arc`, `Rc`, `Box`, `Mutex`, `RefCell` in public APIs unless the user *needs* to know. They couple consumers to your concurrency choices.
- **M-DI-HIERARCHY** — Prefer concrete types → generics (`<T: Trait>`) → trait objects (`dyn Trait`). Reach for `dyn` only when monomorphization cost or heterogeneity demands it.
- **M-ERRORS-CANONICAL-STRUCTS** — Error types are structs (or enums of structs), not raw `String`/`&str`. Implement `std::error::Error`, derive via `thiserror` is fine.
- **M-INIT-BUILDER** — Complex constructors → builder pattern. > ~4 params or many optionals = builder. Avoid Java-style `new(a, b, c, d, e, f, g)`.
- **M-INIT-CASCADED** — Initialization hierarchies cascade: parent builds child by passing context, child does not reach up. One-way data flow.
- **M-SERVICES-CLONE** — Long-lived service handles (clients, pools, registries) are `Clone` (typically via `Arc` internally). Sharing across tasks/threads must not require external wrappers.
- **M-IMPL-ASREF** — Accept `impl AsRef<str>` / `AsRef<Path>` instead of `&str`/`&Path` where flexibility is cheap. Callers pass `String`, `&String`, `PathBuf` without extra ceremony.
- **M-IMPL-RANGEBOUNDS** — Range-taking APIs accept `impl RangeBounds<T>` so callers pass `0..10`, `..=5`, `3..`, etc.
- **M-IMPL-IO** — "Sans-IO" — APIs accept `impl Read` / `impl Write` / `impl AsyncRead` rather than concrete `File`/`TcpStream`. Testable, composable.
- **M-ESSENTIAL-FN-INHERENT** — Core operations are inherent methods, not trait methods. Users should not need to `use FooTrait` to call the obvious method.

### Resilience

- **M-MOCKABLE-SYSCALLS** — Wrap system calls / I/O behind a trait or fn-pointer seam so tests can substitute fakes. No untestable `std::fs` calls buried in business logic.
- **M-TEST-UTIL** — Test helpers exposed for downstream test use are gated behind a `test-util` feature so production builds stay lean.
- **M-STRONG-TYPES** — Avoid primitive obsession. `UserId(u64)` not `u64`. `Url`, `PathBuf`, `Duration` — use the right type family. Compiler checks > runtime checks.
- **M-NO-GLOB-REEXPORTS** — No `pub use foo::*` in libraries. Explicit re-exports only. Glob re-exports leak future symbols silently.
- **M-AVOID-STATICS** — Avoid `static mut` and hidden global state. Pass dependencies explicitly. If you must, use `OnceLock`/`LazyLock` and document the lifetime.

### Building

- **M-OOBE** — Library compiles and basic example runs with zero config after `cargo add`. No required env vars, no required external services for `cargo test` of the no-feature build.
- **M-SYS-CRATES** — `*-sys` crates build with only their declared system-library dependency. No surprise runtime deps.
- **M-FEATURES-ADDITIVE** — Cargo features are purely additive. Enabling a feature must NEVER remove APIs or change behavior of unrelated APIs. No mutually-exclusive features.

## Applications (binaries, not libraries)

- **M-MIMALLOC-APP** — Binaries set `mimalloc` (or `jemalloc`) as the global allocator unless profiling shows the default is fine. Significant throughput wins on multi-threaded workloads.
- **M-APP-ERROR** — Apps may use `anyhow` / `eyre` for fast top-level error plumbing. Libraries must NOT — see `M-ERRORS-CANONICAL-STRUCTS`.

## FFI

- **M-ISOLATE-DLL-STATE** — When shipping cdylibs / loaded into other processes, isolate global state per library load. No assumptions of singleton allocators, runtimes, or TLS.

## Safety

- **M-UNSAFE** — `unsafe` requires a `// SAFETY: ...` comment explaining the invariants being upheld. Avoid `unsafe` unless measurably necessary (FFI, proven perf hot path, sound abstraction over raw primitives).
- **M-UNSAFE-IMPLIES-UB** — Treat any `unsafe` block as a potential UB site. Every precondition listed in the called fn's `# Safety` doc must be locally verified at the call site.
- **M-UNSOUND** — Public safe APIs MUST NOT be soundness-breakable from safe code. If a safe input can trigger UB, the API is broken — fix it, do not document around it.

## Performance

- **M-THROUGHPUT** — Optimize for throughput first. Avoid empty cycles: no polling sleeps, no `yield_now` loops, no spin-without-backoff.
- **M-HOTPATH** — Identify the hot path early, profile it (`cargo flamegraph`, `perf`, `samply`, `tokio-console` for async), then optimize. Don't pre-optimize cold code.
- **M-YIELD-POINTS** — Long-running sync tasks in async contexts must yield periodically (`tokio::task::yield_now`) or move to `spawn_blocking`. Never starve the runtime.

## Documentation

- **M-FIRST-DOC-SENTENCE** — Doc-comment first line: single sentence, ~15 words, no line break. This is what `cargo doc` summarizes.
- **M-MODULE-DOCS** — Every `mod` and crate root has `//!` module-level docs explaining purpose and entry points.
- **M-CANONICAL-DOCS** — Use canonical doc sections in this order when relevant: `# Examples`, `# Errors`, `# Panics`, `# Safety`. Match upstream conventions.
- **M-DOC-INLINE** — `pub use foo::Bar;` re-exports get `#[doc(inline)]` when `Bar` is conceptually part of this module's surface (otherwise readers must chase the link).

## AI

- **M-DESIGN-FOR-AI** — APIs friendly to humans are friendly to agents. Specifically:
  1. Idiomatic — follow Rust API Guidelines so AI matches familiar patterns.
  2. Documented — every module and public item has docs; assume reader has solid but not expert Rust.
  3. Examples — every non-trivial public item has a runnable doctest.
  4. Strong types — no primitive obsession; compiler steers the agent.
  5. Testable — design for fast unit-test iteration loops.
  6. Coverage — high coverage of observable behavior so agents can refactor safely.

## Reinforcement note

The token-suppression goal common in AI-assisted development compounds with `M-DESIGN-FOR-AI`: tight types and high doctest coverage let smaller/cheaper models complete tasks accurately, which is the entire point of a well-designed AI stack.
