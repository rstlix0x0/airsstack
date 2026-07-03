---
type: Rust Module
title: clauders::agent::error
description: AgentError — the single error type crossing the public Agent SDK API, wrapping the protocol-blind ProcessError plus protocol/discovery/control-level failure modes.
tags: [rust, sdk, agent, error-handling]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/error.rs
---

# Schema

```rust
#[non_exhaustive]
pub enum AgentError {
    BinaryNotFound { searched: Vec<PathBuf> },
    BinaryVersionUnsupported { found: String, minimum: String },
    Process(#[from] ProcessError),
    Protocol { detail: String },
    Decode(String),
    ControlRequestFailed { method: String, detail: String },
    TransportClosed,
    Cli { exit_code: Option<i32>, stderr: String },
    CapabilityUnsupported { feature: String },
    Interrupted,
    Timeout,
}
```

`ProcessError` converts via `#[from]` — a raw subprocess-layer failure
(spawn, kill, reap, timeout) always surfaces to callers as `AgentError::Process`.

# Examples

```rust
use clauders::agent::AgentError;
use clauders::agent::process::ProcessError;
let err: AgentError = ProcessError::Timeout.into();
assert!(matches!(err, AgentError::Process(ProcessError::Timeout)));
```

Related: [ProcessError](/crates/clauders/agent/process/error.md),
[CliRuntime discovery/handshake](/crates/clauders/agent/cli/discovery.md)
(producer of `BinaryNotFound`/`BinaryVersionUnsupported`),
[Demux::route](/crates/clauders/agent/cli/demux.md) (producer of `Protocol`),
[protocol::decode_inbound](/crates/clauders/agent/protocol/codec.md)
(producer of `Decode`).

# Citations

1. `crates/clauders/src/agent/error.rs`
