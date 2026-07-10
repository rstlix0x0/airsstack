---
type: Rust Module
title: clauders::agent::runtime::api::convert
description: Pure impedance mapping between the wire Messages API and the agent frame surface — content-block, usage, and stop-reason conversion, plus error folding. No I/O; the unit-test seam of the api runtime.
tags: [rust, sdk, agent, runtime, messages-api, conversion]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/api/convert.rs
---

# Schema

```rust
pub(super) fn content_block(block: &WireBlock) -> AgentBlock;
pub(super) fn usage(u: &WireUsage) -> AgentUsage;
pub(super) fn last_text(blocks: &[WireBlock]) -> String;
pub(super) const fn stop_reason_wire(reason: StopReason) -> &'static str;
pub(super) fn map_wire_error(error: WireError) -> AgentError;

pub(super) use crate::agent::mcp::naming::{declare_name, route}; // re-exported for `tools`
```

`content_block` maps each `messages::content::ContentBlock` variant
(`Text`, `Thinking`, `ToolUse`, `ToolResult`) to its
[`agent::ContentBlock`](/crates/clauders/agent/content.md) counterpart 1:1.
`usage` carries the prompt-cache token counters through when the wire
response reported them (`cache_creation_input_tokens`,
`cache_read_input_tokens`), else `None`. `last_text` concatenates every
`Text` block's text in order — the result-frame summary. `stop_reason_wire`
maps `StopReason` to the same lowercase-underscore vocabulary the CLI
protocol emits (`end_turn`, `max_tokens`, `stop_sequence`, `tool_use`,
`refusal`). `map_wire_error` folds the (Messages-API-shaped)
[wire `Error`](/crates/clauders/error.md) into the (CLI-centric)
[`AgentError`](/crates/clauders/agent/error.md) surface:
`Transport` → `TransportClosed`, `Serde` → `Decode`, everything else →
`Protocol { detail }`.

# Examples

```rust
use clauders::messages::content::{ContentBlock as WireBlock, TextBlock};
# fn conv(_: &WireBlock) {}
let wire = WireBlock::Text(TextBlock::new("hi"));
// content_block(&wire) -> AgentBlock::Text { text: "hi".to_string() }
```

Related: [content::ContentBlock](/crates/clauders/agent/content.md),
[messages/content](/crates/clauders/messages/content.md),
[messages/response (StopReason/Usage)](/crates/clauders/messages/response.md),
[AgentError](/crates/clauders/agent/error.md),
[ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) (the sole
caller, in `drive`/`emit_assistant`/`terminal_result`),
[openrouter::convert](/crates/clauders/agent/runtime/openrouter/convert.md)
(the structural twin for the other native runtime).

# Citations

1. `crates/clauders/src/agent/runtime/api/convert.rs`
