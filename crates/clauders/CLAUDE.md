# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Scoped to `crates/clauders`. The workspace root `CLAUDE.md` still applies — this file adds only what
is specific to this crate.

## Commands

The Definition of Done is owned by the `airsstack-guideline-rust` skill; while working inside this
crate, run it scoped with `-p clauders` instead of `--workspace`:

```bash
cargo fmt --all -- --check
cargo clippy -p clauders --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p clauders --all-features --no-deps
cargo test  -p clauders --all-targets --all-features
cargo test  -p clauders --all-features --doc
```

Use the `--workspace` form before a release or when the change reaches `airs-transport`.

Narrower runs while iterating:

```bash
cargo test -p clauders --test messages_streaming                  # one integration file
cargo test -p clauders --test messages_streaming -- name_of_test  # one test in it
cargo test -p clauders --lib agent::process::                     # unit tests under one module path
```

A changed `insta` snapshot fails the test and writes a `.snap.new` beside the `.snap` in
`tests/snapshots/`. Read the diff, then replace the `.snap` with it. (`cargo-insta` is not installed
here, so `cargo insta review` is unavailable unless you install it.)

Two test targets are deliberately outside the gate:

- `tests/agent_e2e.rs` is `#[ignore]`d *and* env-guarded. It needs a real backend binary:
  `CLAUDERS_AGENT_E2E=1 cargo test -p clauders --all-features --test agent_e2e -- --ignored`
- `tests/compile_fail/*.stderr` are `trybuild` expectations. Regenerate with
  `TRYBUILD=overwrite cargo test -p clauders --test builder_compile` and read the diff — these files
  pin the type-state builder's error messages, so a changed message is a user-visible change.

Examples are registered by name in `Cargo.toml`, so they run from anywhere in the workspace. Messages
API examples need `ANTHROPIC_API_KEY`; agent examples need a `claude` binary 2.0.0+ on `PATH` and no
API key at all, because the Agent SDK drives that binary rather than calling the API.

```bash
ANTHROPIC_API_KEY=sk-... cargo run -p clauders --example 01_hello
cargo run -p clauders --example agent_01_query
```

Adding an example means adding an `[[example]]` block with an explicit `path` — every example lives in
its own directory with a `main.rs` and a `README.md`, which Cargo's autodiscovery does not find.

## Architecture

Three parity pillars, three module trees, no coupling between them:

| Pillar | Module | Talks to | State |
|---|---|---|---|
| Messages API | `src/messages/`, `src/models/` | Anthropic HTTP API via `airs-transport` | implemented |
| Agent SDK | `src/agent/` | the `claude` binary as a subprocess | implemented |
| Managed Agents | — | `/v1/agents`, `/v1/sessions`, … | not started |

The Managed Agents pillar has no code yet (`grep -rniE "managed_?agent|/v1/agents" src/` returns
nothing, while the same grep for `/v1/messages` hits `src/messages/resource.rs`).

### Messages API side

`Client<T = ReqwestTransport>` (`src/client.rs`) is generic over the HTTP transport per the
static-dispatch policy, holds an `Arc<ClientInner<T>>` so cloning is cheap, and hands out resource
handles: `client.messages()` → `MessagesResource`, `client.models()` → `ModelsResource`, and
`client.messages().batches()` → `BatchesResource`. Construction goes through a type-state builder
(`src/builder.rs`, `Missing`/`Present`) so `build()` does not exist until the API key is set.

### Agent SDK side

Four layers, each blind to the one above it — this is the main thing to preserve when editing:

```
agent::client::Client<R: Runtime>   stateful session + live-control ops
        │
agent::runtime  ── port.rs: the Runtime trait (the single seam)
        │         ├── cli/   CliRuntime: discovery, argv, handshake, demux, dispatch
        │         └── mock.rs  MockRuntime (cfg(test)) — replays turns, records control calls
        │
agent::protocol                     JSON frames + line codec; protocol-aware, transport-blind
        │
agent::process                      spawn/supervise/teardown; knows nothing about `claude` or JSONL
```

`agent::process` manages *any* child process — keep binary- and protocol-specific knowledge out of it.
`agent::protocol` turns lines into frames and back and never touches a process. Everything above the
`Runtime` trait is generic over it, which is why session and client logic is testable with no backend
binary present.

`agent::Options` is the only configuration argument, fixed at spawn time and shared by the one-shot
`query()` and `Client::connect`. Mid-session equivalents are methods on `Client`. `agent::Message` is
exhaustive with a `Message::Other` catch-all, so a newer `claude` release emitting an unmodelled frame
cannot fail a turn — preserve that property for any new frame enum.

### Cross-cutting

`src/types/` holds validating newtypes (`ApiKey`, `ModelId`, `MaxTokens`, `Temperature`, `ToolUseId`,
…), each with an `Invalid*` error. Parse at construction; downstream code treats the type as proof. New
domain values belong here, not as bare `String`/`u32` parameters.

`src/prelude.rs` is the one-import path for callers and is export-only. Per the `mod.rs`-export-only
rule, `lib.rs` and every `mod.rs` in this crate are a table of contents — module docs plus `mod`/`pub
use`, no implementation.

## Testing setup

- **Unit tests** are colocated `#[cfg(test)] mod tests` in each `src/*.rs` (the unit-test mandate).
- **`src/test_support.rs`** — `mockall` fake of the `airs_transport::Transport` contract, `cfg(test)`
  only. Consumer-owned, not feature-gated; the crate declares no Cargo features.
- **`tests/messages_*.rs`, `tests/models.rs`, `tests/transport_reqwest.rs`** — `wiremock` against a
  local HTTP server.
- **`tests/agent_process.rs`, `tests/agent_capabilities.rs`** — drive the real
  `src/bin/clauders-agent-testchild.rs` helper binary via `CARGO_BIN_EXE_clauders-agent-testchild`. Its
  flags (`--ignore-eof`, `--spam-stderr`, `--exit-code`, `--fork-grandchild`, `--init-caps`) exist to
  provoke specific subprocess failure modes; extend the child rather than mocking around it.
- **`tests/messages_snapshot.rs`** — `insta` snapshots of serialized request JSON in `tests/snapshots/`.
- **`tests/builder_compile.rs`** — `trybuild` fixtures locking the type-state contracts.

## Parity docs, and their drift

`docs/vision-and-strategy.md` is the authoritative scope statement — read it before adding a surface,
especially §5 (what was removed) and §8 (what may return). The per-pillar gap tables are
`docs/messages-api/feature-parity.md` and `docs/agent-sdk/feature-parity.md`;
`docs/agent-sdk/development-phases.md` records the fixed five-phase Agent SDK split.

**These tables lag the code.** The Agent SDK scorecard still lists the five session filesystem ops as
behind, but `SessionArchive::{list,info,messages,rename,tag}` are implemented in
`src/agent/sessions/archive.rs:59-185`; warm start is in `src/agent/warm.rs`. When a doc and the source
disagree, the source wins — and say so in the reply rather than repeating the table.
