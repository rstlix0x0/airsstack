---
type: Rust Module
title: clauders::agent::protocol
description: Control-protocol wire types and line codec — protocol-aware but transport-blind; describes the JSON frames riding over the subprocess pipes and turns lines into frames and back.
tags: [rust, sdk, agent, protocol, control-protocol]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/protocol/mod.rs
---

Nothing here spawns or signals a process — that is the
[process module](/crates/clauders/agent/process/overview.md)'s job.

# Schema

| Submodule | Concept |
| --- | --- |
| `codec` | [RequestId / RequestIdGen / decode_inbound / encode_line](/crates/clauders/agent/protocol/codec.md) |
| `frames` | [InboundFrame / ControlResponse(Body) / InboundControlRequest / OutboundControlRequest / OutboundControlResponse](/crates/clauders/agent/protocol/frames.md) |

Three inbound frame kinds arrive on the binary's stdout: a **message**
frame (forwarded to the caller's message stream), a **`control_response`**
(matched to a waiter by `request_id`), and an inbound **`control_request`**
(`can_use_tool`/`hook_callback`, answered via a correlated control
response). Outbound, the runtime writes a user-message frame and
`control_request` frames (`interrupt`/`set_model`/…).

# Citations

1. `crates/clauders/src/agent/protocol/mod.rs`
