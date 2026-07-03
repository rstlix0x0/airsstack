---
type: Rust Module
title: clauders::agent::cli::handshake
description: initialize_request / parse_capabilities — builds the SDK's first control request (declaring registered hooks) and tolerantly parses the binary's capability manifest response.
tags: [rust, sdk, agent, cli, handshake]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/cli/handshake.rs
---

# Schema

```rust
pub(super) fn initialize_request(options: &Options, request_id: &str) -> serde_json::Value;
pub(super) fn warn_unsupported_hooks(options: &Options, caps: &Capabilities);
pub(super) fn parse_capabilities(response: &serde_json::Value) -> Capabilities;
```

`initialize_request` builds
`{"type":"control_request","request_id":..,"request":{"subtype":"initialize","system_prompt":..,"hooks"?:..}}`.
Hooks are declared using `Capabilities::default()` (all-unknown) because
caps are not yet known pre-handshake — the binary simply never fires events
it does not support. `warn_unsupported_hooks` re-runs the gating once caps
are known post-handshake, purely to log a developer-facing mismatch warning
(no behavioral effect). `parse_capabilities` is tolerant: an unrecognized
or malformed payload yields the default (empty) manifest so an absent
feature reads as unsupported rather than failing the handshake.

# Examples

```rust
use clauders::agent::Options;
// initialize_request(&Options::builder().system_prompt("hello").build(), "req_0")
// -> {"type":"control_request","request_id":"req_0","request":{"subtype":"initialize","system_prompt":"hello"}}
```

Related: [Capabilities](/crates/clauders/agent/capabilities.md),
[HookRegistry::initialize_payload](/crates/clauders/agent/hooks.md),
[protocol::encode_line/decode_inbound](/crates/clauders/agent/protocol/codec.md),
[CliRuntime::connect](/crates/clauders/agent/cli/runtime.md) (the sole caller,
via its private `handshake()` helper).

# Citations

1. `crates/clauders/src/agent/cli/handshake.rs`
