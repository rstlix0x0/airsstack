---
type: Rust Module
title: clauders::agent::runtime::cli::demux
description: Demux — routes decoded inbound frames to the active turn's message channel or a pending control-response waiter, keyed by correlation id.
tags: [rust, sdk, agent, cli, demultiplexing]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/cli/demux.rs
---

Relocated from `agent/cli/demux.rs` to `agent/runtime/cli/demux.rs` in the
runtime-adapter regroup — see the
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).
Behavior is unchanged by the move.

The reader task decodes each stdout line into an `InboundFrame` and hands it
here. Message frames go to the current turn's channel (cleared when the
terminal `Result` frame arrives); control responses resolve the matching
pending request by id. An unexpected inbound control request is surfaced on
the active turn as a protocol error, since (at this layer) no handler is
registered — that dispatch happens one level up via
[Dispatcher](/crates/clauders/agent/cli/dispatch.md).

# Schema

```rust
pub(super) struct Demux {
    pending: Mutex<HashMap<String, oneshot::Sender<ControlResponseBody>>>,
    turn_sink: Mutex<Option<mpsc::Sender<Result<Message, AgentError>>>>,
}

impl Demux {
    pub(super) fn new() -> Self;
    pub(super) fn set_turn_sink(&self, sink: mpsc::Sender<Result<Message, AgentError>>);
    pub(super) fn register_pending(&self, id: String, waiter: oneshot::Sender<ControlResponseBody>);
    pub(super) fn remove_pending(&self, id: &str);
    pub(super) async fn route(&self, frame: InboundFrame);
    pub(super) async fn fail_turn(&self, error: AgentError);
    pub(super) async fn close(&self);
}
```

`route` behavior: `InboundFrame::Message` forwards to the turn sink and
clears it once the message is a `Result`; `InboundFrame::ControlResponse`
resolves and removes the matching pending waiter; `InboundFrame::ControlRequest`
fails the active turn with `AgentError::Protocol` (this layer has no
handler — see [Dispatcher](/crates/clauders/agent/cli/dispatch.md) for the
layer that actually answers these, now including in-process MCP
`mcp_message` requests). `close()` fails the turn with
`AgentError::TransportClosed` and clears the sink.

Related: [CliRuntime](/crates/clauders/agent/cli/runtime.md) (owns the
`Demux` and spawns `reader_loop`), [protocol frames](/crates/clauders/agent/protocol/frames.md),
[MessageStream](/crates/clauders/agent/stream.md) (fed by the turn sink),
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Citations

1. `crates/clauders/src/agent/runtime/cli/demux.rs`
