---
type: Rust Module
title: clauders::messages
description: Messages API surface — request/response types plus MessagesResource, the entry point for POST /v1/messages, streaming, batches, and token counting.
tags: [rust, sdk, anthropic, messages-api]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/mod.rs
---

Feature-gated module (`messages`, on by default) so request/response types
for `POST /v1/messages` are only compiled when needed. Re-exports every
public type from its submodules so callers import from `clauders::messages::*`
without navigating them directly.

# Schema

| Submodule | Feature gate | Concept |
| --- | --- | --- |
| `content` | `messages` | [ContentBlock / TextBlock / ThinkingBlock](/crates/clauders/messages/content.md) |
| `request` | `messages` | [Role, InputMessage, MessageRequest(Builder)](/crates/clauders/messages/request.md) |
| `resource` | `messages` | [MessagesResource](/crates/clauders/messages/resource.md) |
| `response` | `messages` | [Message, StopReason, Usage](/crates/clauders/messages/response.md) |
| `streaming` | `messages-streaming` | [StreamEvent, MessageStream](/crates/clauders/messages/streaming.md) |
| `tools` | `messages-tools` | [Tool, ToolChoice, ToolUseBlock](/crates/clauders/messages/tools.md) |
| `token_counting` | `messages-token-counting` | [TokenCount](/crates/clauders/messages/token-counting.md) |
| `structured_outputs` | `messages-structured-outputs` | [OutputConfig](/crates/clauders/messages/structured-outputs.md) |
| `batches` | `messages-batches` | [Batches overview](/crates/clauders/messages/batches/overview.md) |

Entry point: `MessagesResource`, obtained via `client.messages()` on
[Client](/crates/clauders/client.md).

# Citations

1. `crates/clauders/src/messages/mod.rs`
