---
type: Rust Crate
title: clauders
description: Unofficial Rust SDK for the Anthropic Claude Messages API, with an optional Agent SDK that drives the `claude` Code CLI as a subprocess.
tags: [rust, sdk, anthropic, claude, http-client, agent]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/lib.rs
---

`clauders` is a Rust crate providing two related but independently-gated surfaces:

1. **Messages SDK** (`messages`, `models`, and friends) — request/response types
   and a `Client<T>` for `POST /v1/messages`, `GET /v1/models`, and related
   Anthropic HTTP endpoints. Generic over the HTTP transport (`airs_transport`),
   defaulting to `reqwest`.
2. **Agent SDK** (`agent` feature) — drives the `claude` Code CLI binary as a
   subprocess over its JSONL control protocol, exposing a session `Client`
   that sends prompts and streams message frames, plus in-loop hooks and
   tool-permission policies.

## Feature flags

| Feature | Default | Depends on | Purpose |
| --- | --- | --- | --- |
| `messages` | yes | — | Request/response types + `MessagesResource` |
| `messages-streaming` | yes | `messages` | SSE streaming via `MessageStream` |
| `messages-tools` | yes | `messages` | Tool (function-calling) types |
| `messages-caching` | yes | — | Prompt-caching fields |
| `transport-reqwest` | yes | — | Default `reqwest`-backed HTTP transport |
| `messages-token-counting` | no | `messages` | `POST /v1/messages/count_tokens` |
| `models` | no | — | `GET /v1/models` resource |
| `messages-batches` | no | `messages` | Message Batches API |
| `messages-structured-outputs` | no | `messages` | JSON-Schema-constrained output |
| `agent` | no | — | Subprocess-backed Agent SDK |
| `__test-mocks` | no | — | Internal-only mock transport/runtime for tests |

## Module map

- [Client](/crates/clauders/client.md), [ClientBuilder](/crates/clauders/builder.md),
  [Config](/crates/clauders/config.md), [Auth](/crates/clauders/auth.md),
  [Error hierarchy](/crates/clauders/error.md), [RetryPolicy](/crates/clauders/retry.md),
  [prelude](/crates/clauders/prelude.md) — the Messages SDK core.
- [messages module](/crates/clauders/messages/overview.md) — request/response
  types and `MessagesResource`.
- [models module](/crates/clauders/models/resource.md) — `ModelsResource`.
- [types module](/crates/clauders/types/api-key.md) — strongly-typed domain
  primitives (newtypes) shared across the crate.
- [agent module](/crates/clauders/agent/overview.md) — the Agent SDK: `Client`,
  `Runtime`, `Options`, hooks, permissions, and the subprocess-driving
  `CliRuntime` (`cli/`, `process/`, `protocol/`, `types/` submodules).

# Citations

1. `crates/clauders/src/lib.rs`
2. `crates/clauders/Cargo.toml`
