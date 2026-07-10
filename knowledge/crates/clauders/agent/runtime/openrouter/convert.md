---
type: Rust Module
title: clauders::agent::runtime::openrouter::convert
description: Pure impedance mapping between the OpenRouter chat-completions wire surface and the agent frame surface — assistant text extraction, usage and finish-reason conversion, error folding. No I/O; the unit-test seam of the openrouter runtime.
tags: [rust, sdk, agent, runtime, openrouter, conversion]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/openrouter/convert.rs
---

# Schema

```rust
pub(super) fn content_text(message: &ResponseMessage) -> String;
pub(super) fn usage(u: &OrUsage) -> AgentUsage;
pub(super) const fn finish_reason_wire(reason: FinishReason) -> &'static str;
pub(super) fn map_or_error(error: OrError) -> AgentError;
```

`content_text` returns the assistant text, or `""` for a tool-only turn
(OpenRouter sends `content: null` when the model emits only tool calls —
`Option<String>::unwrap_or_default()`). `usage` maps only
input/output-token counts (`prompt_tokens`/`completion_tokens`); OpenRouter
usage carries no prompt-cache counters, so the agent-frame `Usage`'s cache
fields are left at their `Default` (`None`) — contrast with
[api::convert::usage](/crates/clauders/agent/runtime/api/convert.md),
which does carry them through. `finish_reason_wire` aligns OpenRouter's
`FinishReason` to the same vocabulary the `api` runtime emits (`Stop` →
`"end_turn"`, `Length` → `"max_tokens"`, `ToolCalls` → `"tool_use"`,
`ContentFilter` → `"refusal"`, plus `Error` → `"error"` and `Unknown` →
`"unknown"`, which have no `api`-runtime counterpart since the Messages
API's `StopReason` has no equivalent variants). `map_or_error` folds an
`openrouter_rs::error::Error` the same way
[api::convert::map_wire_error](/crates/clauders/agent/runtime/api/convert.md)
folds a Messages-API error: `Transport` → `TransportClosed`, `Serde` →
`Decode`, everything else → `Protocol { detail }`.

# Examples

```rust
use openrouter_rs::chat::response::FinishReason;
# fn f(_: FinishReason) {}
// finish_reason_wire(FinishReason::ToolCalls) -> "tool_use"
```

Related: [AgentError](/crates/clauders/agent/error.md),
[message::Usage](/crates/clauders/agent/message.md),
[OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md)
(the sole caller, in `drive`/`emit_assistant`/`terminal_result`),
[api::convert (structural twin)](/crates/clauders/agent/runtime/api/convert.md).

# Citations

1. `crates/clauders/src/agent/runtime/openrouter/convert.rs`
