---
paths:
  - "**/*.rs"
---

# Rust — Unit Test Mandate

Every Rust source file that contains executable logic ships with a colocated `#[cfg(test)] mod tests` block. Integration tests under `tests/` do not satisfy this rule — they complement, not replace, unit tests.

Reinforces `M-STATIC-VERIFICATION` and the test discipline in [[rust-strict-quality]]. Stricter than the Microsoft guideline: this rule defines *where* tests must live, not just *that* they exist.

## The mandate

Each `.rs` file under `src/` MUST contain a `#[cfg(test)] mod tests` block exercising the file's public AND non-trivial private items, UNLESS the file falls under one of the [Exempt file kinds](#exempt-file-kinds) below.

- "Non-trivial" = anything that branches, allocates, parses, formats, validates, masks, classifies, transforms, retries, or implements a non-derived trait. Getter / forwarder / pure-`as_str` are trivial; everything else is not.
- One unit test per *behaviour*, not per *line*. The bar is "would a regression here go unnoticed without this test?" — if yes, the test is required.
- Tests live in the same file as the code they test. Cross-file `mod` tests are not allowed for this purpose. Colocation lets reviewers see code + test in one read and prevents drift when the code moves.

## Exempt file kinds

A file is exempt — and may ship with zero inline tests — ONLY if it falls into one of these categories. The exemption is structural; "I'll add tests later" is not a category.

1. **Export-only modules.** `lib.rs`, `mod.rs`, and any file that contains exclusively `//!` module docs + `mod` / `pub mod` / `pub use` statements. Enforced by [[rust-mod-rs-export-only]]. Examples: `src/lib.rs`, `src/types/mod.rs`, `src/transport/mod.rs`.
2. **Pure type aliases / typedefs.** A file whose only item is a `type Foo = …;` with no `impl` block. Example: `transport/body.rs` (`BodyStream = Pin<Box<dyn Stream<…>>>`).
3. **Pure trait definitions.** A file whose only item is a `pub trait Foo { … }` declaration (no default-method bodies with branching logic, no `impl` blocks for foreign types). Example: `transport/seam.rs` (`HttpTransport` trait).
4. **Generated / macro-expanded test doubles.** Files whose body is dominated by a code-generation macro that itself emits the test surface — `mockall::mock! { … }`, `automock`, similar. Example: `transport/mock.rs`. The integration test that consumes the mock counts as coverage.
5. **`build.rs` and pure-data tables.** Build scripts and files that declare nothing but `const` values with no validation logic.

When the file ships under an exemption, it MUST carry a `//!` line near the top citing which category and why:

```rust
//! Pure trait definition. No inline tests per rust-unit-test-mandate
//! exemption #3 — trait body has no executable logic.
```

This makes the exemption visible to reviewers without requiring grep against the rules file.

## Integration tests are not a substitute

Tests under `tests/` (the cargo integration-test target) drive the crate via its public surface. They are valuable but they do NOT exempt a source file from the inline mandate, because:

- Integration tests cannot reach `pub(crate)` items; logic hidden behind crate-visibility goes uncovered.
- They run against the assembled crate; when one fails, the failing layer is ambiguous.
- They cost more to maintain (a separate binary, full link cycle, longer feedback loop) than a colocated `#[cfg(test)] mod tests`.
- They drift further from the code they exercise — when the code moves, the test that lives in another file often does not.

Concretely: a file like `transport/reqwest_impl.rs` containing a `classify_reqwest_error` function MUST have an inline `#[cfg(test)] mod tests` for `classify_reqwest_error` — even if `tests/transport_reqwest.rs` end-to-end covers the happy path through `send()`. Integration coverage of the call site does not substitute for unit coverage of the classifier.

## Required justification when omitting

If a file is NOT exempt and the author still believes tests are infeasible (rare), the file MUST carry an `//!`-level reason block AND a tracking ticket / issue reference:

```rust
//! Tests deferred. Reason: `try_new` exercises real TLS init; deterministic
//! coverage requires a custom rustls provider. Tracked: <ref>.
```

Code review rejects deferrals without both a reason AND a reference. "Covered by integration tests" is NOT a valid reason — see [previous section](#integration-tests-are-not-a-substitute).

## What the unit test block should cover

For a typical logic-bearing file:

- **Happy path** for every public function. One assertion per behavioural claim.
- **Boundary inputs**: empty, max, zero, NaN/non-finite where the type allows it, single-element, off-by-one.
- **Error paths** for every `Result`-returning function — each error variant constructed.
- **Validation invariants** for parse-don't-validate ctors (per [[rust-strong-types]]) — both accept and reject sides.
- **Serde round-trip** for any type with `#[derive(Serialize, Deserialize)]` that carries semantic content.
- **Trait impls that are not `derive`d** — `Debug` masking, `Display` formatting, `PartialEq` quirks, hand-rolled `Hash`.
- **Panic-on-bypass** paths where a struct literal can violate an invariant the constructor guards (per the `RetryPolicy::backoff` NaN-guard pattern from Phase 4).

For type-state builders, the runtime transitions (`Missing → Present` data movement) are the unit-test scope. The *compile-time* refusal of `build()` without required fields is locked by trybuild compile-fail fixtures (per `M-STATIC-VERIFICATION`) — those live under `tests/compile_fail/`, complementary to unit tests, not substituting for them.

## Reviewer checklist

Reject the change if:

- A new source file under `src/` has no `#[cfg(test)] mod tests` block AND carries no exemption header citing one of the five categories.
- A file claims an exemption but does not match the category (e.g., a file with an `impl` block claiming "export-only").
- A `// reason:` deferral block omits the tracking reference.
- An existing file gains a non-trivial `pub` / `pub(crate)` function without a matching test in the colocated block.
- Tests live in a sibling file (e.g., `foo_tests.rs`) when they could colocate. Colocation is mandatory.
- A test uses `#[ignore]` without an inline reason + tracking link (already covered by [[rust-strict-quality]]; called out here because ignored tests in the unit block are a common loophole).

## Rationale

Three concrete failure modes this rule prevents, all observed in earlier phases:

1. **Hidden classifier logic.** `classify_reqwest_error` in `transport/reqwest_impl.rs` shipped without inline tests because the wiremock integration test exercised one path through it. The other classification branches went unverified at the unit level — a regression in the source-chain heuristic would only surface as a misclassified error category in production.
2. **Silent invariant erosion.** `auth.rs` ships a `Debug` impl whose entire purpose is masking a secret. Without an inline `Debug` test, a future derive-macro change or refactor could quietly start leaking the key, and no integration test would catch it because integration tests don't typically format `Auth`.
3. **Accessor drift.** `client.rs` accessors (`Client::config`, `Client::auth`, `Client::ref_count`) are trivial individually, but they form the contract that downstream sub-resources rely on. Inline tests pin the contract to the file that owns it.

The cost of an inline `#[cfg(test)] mod tests` block is minutes. The cost of a regression caught only when a downstream feature fails is hours of bisecting through unrelated code.

## Interaction with other rules

- [[rust-mod-rs-export-only]] — supplies exemption #1; `mod.rs` / `lib.rs` carry no logic to test.
- [[rust-strict-quality]] — covers the DoD command set including `cargo test`. This rule adds the *placement* requirement.
- [[rust-strong-types]] — newtype validating constructors are exactly the "parse-don't-validate" tests this rule mandates.
- [[rust-doc-comment-discipline]] — doctests on public items remain required by `M-DESIGN-FOR-AI`; doctests and unit tests are complementary, not substitutes for each other.
