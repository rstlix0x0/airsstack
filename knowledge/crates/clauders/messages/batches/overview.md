---
type: Rust Module
title: clauders::messages::batches
description: Message Batches API surface — submit, poll, list, cancel, and stream results for asynchronous batches of message requests.
tags: [rust, sdk, anthropic, messages-api, batches]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/batches/mod.rs
---

Feature-gated submodule of [messages](/crates/clauders/messages/overview.md)
(`messages-batches`, depends on `messages`, not enabled by default) —
isolated because most callers never submit batch workloads.

# Schema

| Submodule | Concept |
| --- | --- |
| `types` | [Wire types](/crates/clauders/messages/batches/types.md): `BatchRequest`, `Batch`, `BatchStatus`, `DeletedMessageBatch`, … |
| `resource` | [BatchesResource](/crates/clauders/messages/batches/resource.md) — HTTP dispatch |
| `results` | [BatchResultStream](/crates/clauders/messages/batches/results.md) — JSONL results streaming |

Entry point: `BatchesResource`, obtained via `client.messages().batches()`.

# Citations

1. `crates/clauders/src/messages/batches/mod.rs`
