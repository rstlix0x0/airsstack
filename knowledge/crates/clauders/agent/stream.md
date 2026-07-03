---
type: Rust Module
title: clauders::agent::stream
description: MessageStream — the boxed Stream<Item = Result<Message, AgentError>> every session surface returns, produced from a tokio mpsc receiver by the ReceiverStream adapter.
tags: [rust, sdk, agent, streaming]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/stream.rs
---

`ReceiverStream` bridges `Receiver::poll_recv` to the `futures_core::Stream`
contract directly, so no external stream-adapter crate is needed.

# Schema

```rust
pub type MessageStream = Pin<Box<dyn Stream<Item = Result<Message, AgentError>> + Send>>;

pub(crate) struct ReceiverStream { rx: Receiver<Result<Message, AgentError>> }
impl ReceiverStream {
    pub(crate) const fn new(rx: Receiver<...>) -> Self;
    pub(crate) fn boxed(self) -> MessageStream;
}
```

The stream ends when the producing side closes its channel — typically
after a `Result` frame, per [Demux::route](/crates/clauders/agent/cli/demux.md)'s
clear-on-result behavior.

# Examples

```rust,no_run
use futures_util::StreamExt;
# async fn example(mut stream: clauders::agent::MessageStream) {
while let Some(item) = stream.next().await {
    let _ = item;
}
# }
```

Related: [Client::query](/crates/clauders/agent/client.md),
[Runtime::run](/crates/clauders/agent/runtime.md),
[Demux](/crates/clauders/agent/cli/demux.md) (installs/clears the turn
sink feeding this stream), [MockRuntime::run](/crates/clauders/agent/mock.md).

# Citations

1. `crates/clauders/src/agent/stream.rs`
