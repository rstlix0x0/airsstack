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
the whole workspace with `cargo test --workspace --all-targets --all-features` before a release. Every
test run carries `--all-features` — a default-feature run is not a gate (see the caution below).

```bash
cargo fmt --check
cargo build --all-features                        # zero warnings (treat warnings as errors)
cargo clippy --all-features --all-targets -- -D warnings  # lints test/example/bench code too
cargo test --all-features --all-targets           # unit + integration + examples as tests
cargo test --all-features --doc                   # doctests (--all-targets excludes them)
cargo doc --no-deps --all-features                # zero rustdoc warnings
```

Rules of the gate:

- **Zero warnings** from build, clippy, and rustdoc. A warning is a failure.
- **`--all-targets` is mandatory for clippy and the test run.** Without it, clippy and `cargo test`
  skip test/example/bench targets and their `#[cfg(test)]` modules — so a lint or failure that lives
  only in test code passes silently. The zero-warning bar covers test code too.
- **Doctests count, and need a separate `--doc` run.** `--all-targets` *excludes* doctests, so they
  are invoked explicitly with `cargo test --all-features --doc`. A failing doctest fails the gate.
- **`--all-features` is mandatory for the test run, never optional.** Plain `cargo test` /
  `cargo test -p <crate>` / `cargo test --workspace` compiles only the default features and
  **silently skips** every `#[cfg(feature = "…")]`-gated test (e.g. the `__test-mocks` mock and
  integration tests). A green default-feature run is NOT a passing gate; only `--all-features` runs
  count.
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
