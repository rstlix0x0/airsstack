---
type: Rust Module
title: clauders::agent::protocol::codec
description: Line framing and (de)serialization for the control protocol — RequestId/RequestIdGen mint correlation ids; decode_inbound and encode_line convert between JSON lines and frames.
tags: [rust, sdk, agent, protocol, codec]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/protocol/codec.rs
---

Inbound lines (already newline-split by
[process::pipes::StdoutLines](/crates/clauders/agent/process/pipes.md)) are
parsed into an `InboundFrame`; outbound frames are serialized to a single
newline-terminated JSON line.

# Schema

```rust
pub struct RequestId(String); // "req_<n>"
impl RequestId {
    pub fn generator() -> RequestIdGen;
    pub fn as_str(&self) -> &str;
}

pub struct RequestIdGen { counter: Arc<AtomicU64> } // cheap to clone, shared counter
impl RequestIdGen { pub fn next(&self) -> RequestId; }

pub fn decode_inbound(line: &str) -> Result<InboundFrame, AgentError>;
pub fn encode_line<T: Serialize>(frame: &T) -> Result<String, AgentError>;
```

`decode_inbound` errors as `AgentError::Decode`; `encode_line` errors as
`AgentError::Protocol`. `RequestIdGen` mints process-local, monotonically
increasing ids.

# Examples

```rust
use clauders::agent::protocol::RequestId;
let gen = RequestId::generator();
let a = gen.next();
let b = gen.next();
assert_ne!(a.as_str(), b.as_str());
assert!(a.as_str().starts_with("req_"));
```

Related: [protocol frames](/crates/clauders/agent/protocol/frames.md),
[CliRuntime::send_control](/crates/clauders/agent/cli/runtime.md) (mints a
`RequestId` per control request), [AgentError::Decode/Protocol](/crates/clauders/agent/error.md).

# Citations

1. `crates/clauders/src/agent/protocol/codec.rs`
