---
name: rust-guidelines
description: Use when writing or editing Rust code or Cargo manifests — supplies the Rust Definition-of-Done (the exact cargo command gate every change must pass) and an index of engineering rules (strong types, static dispatch, module hygiene, doc discipline, unit-test mandate, the Microsoft pragmatic guidelines, and workspace layout). Read the matching reference before applying a rule.
---

# Rust Guidelines

Engineering rules and the Definition-of-Done for Rust work. `SKILL.md` is the gate and the map;
the detail lives in `references/` and is read on demand.

## Definition of Done (the gate)

Every Rust change must pass ALL of the following before it is considered complete. Zero warnings is a
hard bar, not a target. Scope the runs to the crate you touched with `-p <crate> --all-features`; run
the whole workspace with `cargo test --workspace --all-features` before a release. Every test run
carries `--all-features` — a default-feature run is not a gate (see the caution below).

```bash
cargo fmt --check
cargo build --all-features          # zero warnings (treat warnings as errors)
cargo clippy --all-features -- -D warnings
cargo test --all-features           # all tests AND doctests green
cargo doc --no-deps --all-features  # zero rustdoc warnings
```

Feature combinatorics (only if the crate is feature-gated):

```bash
cargo hack check --each-feature     # compile-only guard across feature combinations
```

Rules of the gate:

- **Zero warnings** from build, clippy, and rustdoc. A warning is a failure.
- **Doctests count.** `cargo test` must exercise doctests; a failing doctest fails the gate.
- **`--all-features` is mandatory for the test run, never optional.** Plain `cargo test` /
  `cargo test -p <crate>` / `cargo test --workspace` compiles only the default features and
  **silently skips** every `#[cfg(feature = "…")]`-gated test (e.g. the `__test-mocks` mock and
  integration tests). A green default-feature run is NOT a passing gate; only `--all-features` runs
  count. `cargo hack check --each-feature` is compile-only and does not run any test, so it does not
  substitute for the `--all-features` test run.
- **One `--all-features` test run** exercises all feature-gated logic; `cargo hack check --each-feature`
  is the compile-only combination guard. You do NOT need a powerset test matrix or a
  `--no-default-features` test run for the gate — compile coverage of each feature is sufficient.
- **Scope to the touched crate** with `-p <crate>` during development; widen to the full workspace
  before release.
- No change lands with a `#[allow(...)]` added to silence the gate. Use `#[expect(...)]` with a reason
  when a suppression is genuinely temporary (it auto-fails once unneeded). See the doc-comment and
  strict-quality references.

## Reference index

Read the one that matches your task:

- `references/strict-quality.md` — the full pass/fail bar: zero-warning policy, the DoD command set in
  depth, what "green" means for tests and doctests.
- `references/strong-types.md` — no primitive obsession: newtype domain values, parse-don't-validate at
  construction, type-state builders for required fields and ordered lifecycles, no `bool` params for
  semantic flags.
- `references/static-dispatch.md` — prefer generics over `Box<dyn Trait>`; the narrow justified
  exceptions; why `Arc<Inner>` for cheap-`Clone` services is not a trait-object pattern.
- `references/mod-rs-export-only.md` — `mod.rs`/`lib.rs` are table-of-contents only (module docs +
  `mod`/`pub use`); implementation lives in sibling files named after the item.
- `references/doc-comment-discipline.md` — rustdoc and `//` comments target downstream engineers; no
  internal planning paths, plan/phase identifiers, workflow vocabulary, or AI/agent names in source.
- `references/unit-test-mandate.md` — every logic-bearing `src/*.rs` ships colocated
  `#[cfg(test)] mod tests`; the five structural exemptions and how to cite them; integration tests
  complement but do not substitute.
- `references/microsoft-guidelines.md` — the Microsoft Pragmatic Rust Guidelines (the `M-*` rules) this
  ruleset builds on.
- `references/workspace.md` — workspace layout, root vs member `Cargo.toml`, centralized
  `[workspace.package|dependencies|lints]`, naming, publishing order.

When a reference cross-mentions another rule, read that reference too if it bears on your change.
