# airsstack-guideline-rust

Rust engineering guidelines and a strict Definition-of-Done for Claude Code, delivered as a single
lazily-loaded skill: `airsstack-guideline-rust:rust-guidelines`.

The skill loads only when you (or an agent) are editing Rust / Cargo files, so it costs no context
the rest of the time. The `airsstack` plugin's execution agents invoke it to obtain the Rust DoD and
rules when they touch Rust code; it is equally useful on its own.

## Install

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack-guideline-rust@airsstack
```

No agents, no hooks — knowledge only.

## What it provides

- **Definition of Done** — the pass/fail gate for every Rust change: `cargo fmt`, `build`, `clippy`,
  `test`, and `doc` all `--all-features` with zero warnings, plus `cargo hack check --each-feature`
  for feature combinatorics. Scope runs to the touched crate with `-p <crate>`.
- **Rule references** (progressive disclosure, loaded on demand): strict-quality, strong-types,
  static-dispatch, mod.rs-export-only, doc-comment-discipline, unit-test-mandate, Microsoft pragmatic
  guidelines, and workspace layout.

## How it pairs with `airsstack`

The main plugin's agents are language-agnostic; they look up the DoD and rules from whichever
`*-guidelines` skill is installed. Install this alongside `airsstack` to give those agents a Rust
DoD; omit it for non-Rust projects (the agents degrade gracefully).

## License

Apache-2.0. See [LICENSE](./LICENSE).
