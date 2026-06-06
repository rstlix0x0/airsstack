# Rust — Strict Quality Bar

Non-negotiable quality gates for every Rust change. A change is **not done** until every command in [Definition of Done](#definition-of-done) exits `0` with no warnings on a clean checkout.

This rule reinforces `M-STATIC-VERIFICATION` and `M-LINT-OVERRIDE-EXPECT` from the Microsoft guidelines reference but is stricter: those describe the toolset, this defines the pass/fail threshold.

## Zero warnings — everywhere

- `cargo build` MUST produce zero warnings on every target.
- `cargo clippy` MUST produce zero warnings (including pedantic categories enabled by `clippy.toml`).
- `cargo doc` MUST produce zero warnings (no broken intra-doc links, no missing docs on public items).
- `rustdoc` warnings count as build warnings. Treat `[broken_intra_doc_links]`, `[missing_docs]`, `[private_intra_doc_links]` as errors.

### How to enforce

Prefer **flag-based** enforcement over source-level `#![deny(warnings)]`:

- CI passes `RUSTFLAGS="-D warnings"` and `RUSTDOCFLAGS="-D warnings"`.
- `cargo clippy -- -D warnings` in CI and pre-commit.
- Per-crate `lints` table in `Cargo.toml` (Cargo ≥ 1.74) is acceptable for repo-wide lint policy:

  ```toml
  [workspace.lints.rust]
  unsafe_code = "deny"
  missing_docs = "warn"
  rust_2018_idioms = { level = "warn", priority = -1 }

  [workspace.lints.clippy]
  all = { level = "warn", priority = -1 }
  pedantic = { level = "warn", priority = -1 }
  nursery = { level = "warn", priority = -1 }
  cargo = { level = "warn", priority = -1 }
  ```

  Then each crate opts in with `[lints] workspace = true`.

- Do NOT use `#![deny(warnings)]` in source. Toolchain bumps introduce new lints; a source-level deny turns every `cargo update` of `rustc` into a breaking build. Flag-based denials live in CI / `Cargo.toml` lints table where they can be relaxed for a release without touching code.

### Lint suppressions

Suppression of any lint requires `#[expect(lint_name, reason = "...")]` (per `M-LINT-OVERRIDE-EXPECT`). `#[allow]` is reserved for cases where the lint fires conditionally (feature-gated code) and `#[expect]` would itself warn. Every suppression carries a `reason = "..."` string; reviews reject suppressions without one.

## All tests green — including doctests

Every PR MUST pass:

- `cargo test --workspace --all-targets --all-features` — unit, integration, examples, benches as tests. One all-features run exercises every feature's *logic* (including non-default features) without the combinatoric cost of testing each feature set separately.
- `cargo test --workspace --all-features --doc` — doctests. `--all-targets` does NOT include doctests; they must be invoked explicitly.
- `cargo hack check --each-feature --no-dev-deps` — **compile-only** isolation guard. Linear (one compile per feature, plus the no-default cell), no link, no test-run. Catches the bug class that per-feature-set *testing* used to catch — under-gated items, default-feature leakage, a feature broken in isolation — because those are compile errors, not test failures.

**Feature-combination *testing* is deliberately NOT required.** Do not run `cargo test --no-default-features`, `cargo hack test --each-feature`, or `cargo hack test --feature-powerset` as a gate — they re-run the whole suite under every feature set and the wall-clock cost (a full rebuild + link + run per set) is not worth the marginal signal over the single all-features test run plus the cheap `--each-feature` *check*. The check compiles each cell; it does not execute tests per cell.

Skipped or ignored tests need a `// reason: ...` comment and a tracking issue link. `#[ignore]` without justification fails review.

### Doctest requirements

- Every public item with non-trivial behavior has at least one doctest demonstrating the happy path (`M-DESIGN-FOR-AI` reinforces this).
- Doctests are real tests — they compile and run. `no_run` is permitted only for examples that hit external resources (network, FS outside `tempfile`); `ignore` requires an inline reason.
- Doctest setup that doesn't belong in user-facing docs goes in hidden lines (`# `), not removed.

## Definition of Done

A Rust change is complete when ALL of these pass on a clean checkout (no cached `target/`):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo test  --workspace --all-targets --all-features   # single all-on test run
cargo test  --workspace --all-features --doc           # doctests
cargo hack check --each-feature --no-dev-deps          # compile-only isolation guard (NOT a test)
```

Scope note: when a change touches a single crate, scope the test runs to it
(`-p <crate>`) instead of `--workspace` — there is no value re-testing an
untouched member. Reserve the `--workspace` form for changes that actually
cross crates.

Optional but recommended before merging significant changes:

```bash
cargo audit                                   # known-vuln deps
cargo +nightly udeps --workspace              # unused deps
cargo +nightly miri test --workspace          # if any unsafe touched
```

## Reviewer checklist

Reject the change if:

- Any command above fails or warns.
- New `#[allow(...)]` appears without `#[expect(..., reason = "...")]` rewrite justification.
- New public item lacks a doctest or `# Examples` block.
- `#[ignore]` added without reason + tracking link.
- `unwrap()` / `expect()` / `panic!()` added in library code without a `# Panics` doc section justifying it (apps may use them more liberally per `M-APP-ERROR`).
- New `unsafe` block without `// SAFETY: ...` comment (`M-UNSAFE`).

## Local automation

Wire the Definition of Done commands into:

- A pre-commit hook (or `cargo-husky`) running `fmt --check` + `clippy -D warnings`.
- A pre-push hook running the full test suite including doctests.
- CI runs every command on every PR; merge is blocked on any failure.

The cost of catching a warning locally is seconds; the cost of catching it in CI is a round-trip; the cost of merging it is technical debt. Pay the cheapest one.
