---
type: Rust Module
title: clauders::agent::runtime::cli::handshake
description: initialize_request / parse_capabilities — builds the SDK's first control request (system prompt lowered via SystemPromptConfig::native_text, plus registered hooks) and tolerantly parses the binary's capability manifest response.
tags: [rust, sdk, agent, cli, handshake, system-prompt]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/cli/handshake.rs
---

Relocated from `agent/cli/handshake.rs` to `agent/runtime/cli/handshake.rs`
in the runtime-adapter regroup — see the
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Schema

```rust
pub(super) fn initialize_request(options: &Options, request_id: &str) -> serde_json::Value;
pub(super) fn warn_unsupported_hooks(options: &Options, caps: &Capabilities);
pub(super) fn parse_capabilities(response: &serde_json::Value) -> Capabilities;
```

`initialize_request` builds
`{"type":"control_request","request_id":..,"request":{"subtype":"initialize","system_prompt":..,"hooks"?:..}}`.
The `system_prompt` field is `options.system_prompt.native_text()` —
[`SystemPromptConfig::native_text`](/crates/clauders/agent/system-prompt.md):
`None` → the field is `null`; `Text(s)` → `s`; `Preset { append, .. }` →
`append` (the preset's `claude_code` base itself is never sent here — the
base is CLI-resident and is instead requested via the `--append-system-prompt`
argv flag built by [argv::build_argv](/crates/clauders/agent/cli/argv.md);
this handshake field only carries appended/overridden text).
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

// initialize_request(&Options::builder().system_prompt_preset(Some("appended".into()), false).build(), "req_0")
// -> request.system_prompt == "appended" (the claude_code base is requested via argv, not this field)
```

Related: [Capabilities](/crates/clauders/agent/capabilities.md),
[SystemPromptConfig](/crates/clauders/agent/system-prompt.md),
[HookRegistry::initialize_payload](/crates/clauders/agent/hooks.md),
[protocol::encode_line/decode_inbound](/crates/clauders/agent/protocol/codec.md),
[CliRuntime::connect](/crates/clauders/agent/cli/runtime.md) (the sole caller,
via its private `handshake()` helper),
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Citations

1. `crates/clauders/src/agent/runtime/cli/handshake.rs`
2. `crates/clauders/src/agent/system_prompt.rs`
