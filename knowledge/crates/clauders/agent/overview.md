---
type: Rust Module
title: clauders::agent
description: Claude Agent SDK surface — drives the `claude` Code CLI binary as a subprocess over its JSONL control protocol, exposing a session Client, Runtime port, Options, hooks, and permission policies.
tags: [rust, sdk, agent, claude-code, subprocess, cli]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/mod.rs
---

Gated behind the `agent` feature (pulls in `nix` on Unix and
`tokio/process`). Split into a data layer (error hierarchy, strong types,
message/content frames, the capability manifest, `Options`, the
control-protocol codec) and an async `Runtime`/`Client` layer.

# Schema

| Submodule | Concept |
| --- | --- |
| `capabilities` | [Capabilities / HookEvent](/crates/clauders/agent/capabilities.md) — negotiated feature manifest |
| `client` | [Client / AgentClientBuilder / query()](/crates/clauders/agent/client.md) — the stateful session handle |
| `content` | [ContentBlock](/crates/clauders/agent/content.md) — agent message content blocks |
| `error` | [AgentError](/crates/clauders/agent/error.md) — the public error type |
| `hooks` | [Hook / HookRegistry](/crates/clauders/agent/hooks.md) — in-loop callbacks |
| `message` | [Message / AssistantMessage / ResultMessage / …](/crates/clauders/agent/message.md) |
| `mock` | [MockRuntime](/crates/clauders/agent/mock.md) — subprocess-free test double (`__test-mocks`) |
| `options` | [Options / OptionsBuilder](/crates/clauders/agent/options.md) — session configuration |
| `permissions` | [PermissionMode / PermissionPolicy](/crates/clauders/agent/permissions.md) — tool-gating |
| `runtime` | [Runtime trait](/crates/clauders/agent/runtime.md) — the single seam of the SDK core |
| `stream` | [MessageStream](/crates/clauders/agent/stream.md) — the boxed message stream type |
| `cli` | [CliRuntime overview](/crates/clauders/agent/cli/overview.md) — subprocess-backed `Runtime` impl |
| `process` | [Process management overview](/crates/clauders/agent/process/overview.md) — protocol-blind subprocess lifecycle |
| `protocol` | [Control-protocol overview](/crates/clauders/agent/protocol/overview.md) — wire frames and line codec |
| `types` | [Agent-specific types overview](/crates/clauders/agent/types/overview.md) — `Prompt`, `SessionId`, MCP types |

Everything above the [Runtime trait](/crates/clauders/agent/runtime.md) —
namely [Client](/crates/clauders/agent/client.md) — is concrete and generic
over it; two implementors exist: the subprocess-backed
[CliRuntime](/crates/clauders/agent/cli/runtime.md) (default) and
[MockRuntime](/crates/clauders/agent/mock.md) (test double).

# Citations

1. `crates/clauders/src/agent/mod.rs`
2. `crates/clauders/Cargo.toml`
