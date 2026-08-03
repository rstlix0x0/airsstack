# Rust — Strict Quality Bar

Non-negotiable quality gates for every Rust change. A change is **not done** until every command in [Definition of Done](#definition-of-done) exits `0` with no warnings on a clean checkout.

This rule reinforces `M-STATIC-VERIFICATION` and `M-LINT-OVERRIDE-EXPECT` from the Microsoft guidelines reference but is stricter: those describe the toolset, this defines the pass/fail threshold.

## Zero warnings — everywhere

- Compilation MUST produce zero warnings on every target. `cargo clippy -- -D warnings` enforces
  this for rustc warnings as well as clippy lints, which is why the gate has no separate
  `cargo build` step.
- `cargo clippy` MUST produce zero warnings (including the pedantic and nursery categories the
  workspace lint table enables).
- `cargo doc` MUST produce zero warnings (no broken intra-doc links, no missing docs on public items).
- `rustdoc` warnings count as build warnings. Treat `[broken_intra_doc_links]`, `[missing_docs]`, `[private_intra_doc_links]` as errors.

### How to enforce

Prefer **flag-based** enforcement over source-level `#![deny(warnings)]`:

- CI passes `RUSTFLAGS="-D warnings"` and `RUSTDOCFLAGS="-D warnings"`.
- `cargo clippy -- -D warnings` in CI and pre-commit.
- A `[workspace.lints]` table in the root `Cargo.toml` (Cargo ≥ 1.74) is the repo-wide lint policy;
  each crate opts in with `[lints] workspace = true`. This workspace's root manifest is the
  authoritative list — read `Cargo.toml` rather than any copy of it. Notable entries:
  `unsafe_code = "forbid"`, `unused_must_use = "deny"`, and on the clippy side `unwrap_used`,
  `panic`, and `dbg_macro` all `"deny"`, with `pedantic` and `nursery` at `"warn"`.

- Do NOT use `#![deny(warnings)]` in source. Toolchain bumps introduce new lints; a source-level deny turns every `cargo update` of `rustc` into a breaking build. Flag-based denials live in CI / `Cargo.toml` lints table where they can be relaxed for a release without touching code.

### Lint suppressions

Suppression of any lint requires `#[expect(lint_name, reason = "...")]` (per `M-LINT-OVERRIDE-EXPECT`). `#[allow]` is reserved for the case where the lint fires only under some build configurations, so `#[expect]` would itself warn in the others — a situation this featureless workspace does not currently produce. Every suppression carries a `reason = "..."` string; reviews reject suppressions without one.

## All tests green — including doctests

Every PR MUST pass:

- `cargo test --workspace --all-targets --all-features` — unit, integration, examples, benches as tests. One all-features run exercises every feature's *logic* (including non-default features) without the combinatoric cost of testing each feature set separately.
- `cargo test --workspace --all-features --doc` — doctests. `--all-targets` does NOT include doctests; they must be invoked explicitly.

Skipped or ignored tests need a `// reason: ...` comment and a tracking issue link. `#[ignore]` without justification fails review.

### Doctest requirements

- Every public item with non-trivial behavior has at least one doctest demonstrating the happy path (`M-DESIGN-FOR-AI` reinforces this).
- Doctests are real tests — they compile and run. `no_run` is permitted only for examples that hit external resources (network, FS outside `tempfile`); `ignore` requires an inline reason.
- Doctest setup that doesn't belong in user-facing docs goes in hidden lines (`# `), not removed.

## Definition of Done

The command set lives in `../SKILL.md` § Definition of Done (the gate) — one canonical copy, so this
file states the policy and never restates the commands. A change is complete when every command
there exits `0` with no warnings on a clean checkout (no cached `target/`).

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
