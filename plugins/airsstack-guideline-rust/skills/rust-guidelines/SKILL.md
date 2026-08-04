---
name: rust-guidelines
description: Use when writing or editing Rust code or Cargo manifests — supplies the Rust Definition-of-Done (the exact cargo command gate every change must pass) and an index of engineering rules (strong types, static dispatch, module hygiene, doc discipline, unit-test mandate, the Microsoft pragmatic guidelines, and workspace layout). Read the matching reference before applying a rule.
---

# Rust Guidelines

Engineering rules and the Definition-of-Done for Rust work. `SKILL.md` is the gate and the map;
the detail lives in `references/` and is read on demand.

## Definition of Done (the gate)

This block is the gate — the single canonical command set. Every command must exit `0` with no
warnings on a clean checkout before a change is complete. `references/strict-quality.md` explains the
policy behind it; it does not restate the commands.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo test  --workspace --all-targets --all-features   # unit + integration + examples + benches
cargo test  --workspace --all-features --doc           # doctests (--all-targets excludes them)
```

Rules of the gate:

- **Zero warnings** from clippy, rustdoc, and the test build. A warning is a failure.
- **`RUSTDOCFLAGS="-D warnings"` is what makes the doc step a gate.** Plain
  `cargo doc` prints rustdoc warnings and still exits `0`, so without the flag the step asserts a bar
  it does not enforce.
- **There is deliberately no separate `cargo build`.** `cargo clippy -- -D warnings` promotes plain
  rustc warnings to errors, not only clippy lints, and `cargo test --all-targets` performs the full
  codegen-and-link build. A `cargo build` step adds compile time without adding coverage.
- **`--all-targets` is mandatory for clippy and the test run.** Without it, clippy and `cargo test`
  skip test/example/bench targets and their `#[cfg(test)]` modules — so a lint or failure that lives
  only in test code passes silently. The zero-warning bar covers test code too.
- **Doctests count, and need a separate `--doc` run.** `--all-targets` *excludes* doctests, so they
  are invoked explicitly. A failing doctest fails the gate.
- **`--all-features` is carried for forward-safety, not because a feature gate exists.** No crate in
  this workspace declares Cargo `[features]`, so today the flag equals the default build. Keep it in
  every command so the gate stays correct the day a feature is added.
- **Scope to the touched crate** with `-p <crate>` in place of `--workspace` while developing a
  single-crate change; use the `--workspace` form for cross-crate changes and before a release.
- No change lands with a `#[allow(...)]` added to silence the gate. Use `#[expect(...)]` with a reason
  when a suppression is genuinely temporary (it auto-fails once unneeded). See the doc-comment and
  strict-quality references.

## Reference index

Read the one that matches your task:

- `references/strict-quality.md` — the policy behind the gate above: zero-warning scope, flag-based
  vs source-level enforcement, lint-suppression rules, what "green" means for tests and doctests.
- `references/strong-types.md` — no primitive obsession: newtype domain values, parse-don't-validate at
  construction, type-state builders for required fields and ordered lifecycles, no `bool` params for
  semantic flags.
- `references/static-dispatch.md` — prefer generics over `Box<dyn Trait>`; the narrow justified
  exceptions; why `Arc<Inner>` for cheap-`Clone` services is not a trait-object pattern.
- `references/mod-rs-export-only.md` — `mod.rs`/`lib.rs` are table-of-contents only (module docs +
  `mod`/`pub use`); implementation lives in sibling files named after the item.
- `references/modularity.md` — one responsibility per unit, one canonical type per concept: the
  God-object gate and the duplicate-type gate.
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
