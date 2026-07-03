---
type: Rust Module
title: clauders::agent::content
description: ContentBlock — an exhaustive enum of content blocks (text, thinking, tool_use, tool_result, server_tool_use) making up an agent assistant or user message.
tags: [rust, sdk, agent, content-blocks]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/content.rs
---

Exhaustive: the compiler forces consumers to handle every block kind, so a
new message shape cannot be silently dropped. Unknown *fields* within a
known block are tolerated (forward-compat); an unknown *block type*
surfaces as a deserialize error mapped to `AgentError::Protocol`.

# Schema

```rust
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: serde_json::Value, is_error: bool },
    ServerToolUse { id: String, name: String, input: serde_json::Value },
}
```

# Examples

```rust
use clauders::agent::ContentBlock;
let block: ContentBlock = serde_json::from_str(r#"{"type":"text","text":"hi"}"#).unwrap();
assert!(matches!(block, ContentBlock::Text { text } if text == "hi"));
```

Related: [AssistantMessage](/crates/clauders/agent/message.md) (carries a
`Vec<ContentBlock>`), [Messages API ContentBlock](/crates/clauders/messages/content.md)
(the independent, strictly-typed analogue used by the non-agent SDK).

# Citations

1. `crates/clauders/src/agent/content.rs`
