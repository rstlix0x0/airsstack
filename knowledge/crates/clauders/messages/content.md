---
type: Rust Module
title: clauders::messages::content
description: ContentBlock — tagged union of message content shapes (text, thinking, tool use/result) shared by Messages API requests and responses.
tags: [rust, sdk, anthropic, messages-api, content-blocks]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/content.rs
---

Content-block types are kept in their own module so each shape can be
extended independently of request assembly ([request.rs](/crates/clauders/messages/request.md))
and response decoding ([response.rs](/crates/clauders/messages/response.md)).

# Schema

```rust
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextBlock),
    Thinking(ThinkingBlock),
    ToolUse(tools::ToolUseBlock),      // feature messages-tools
    ToolResult(tools::ToolResultBlock), // feature messages-tools
}

pub struct TextBlock {
    pub text: String,
    pub cache_control: Option<CacheControl>, // feature messages-caching
}

pub struct ThinkingBlock {
    pub thinking: String,
    pub signature: Option<String>, // omitted from wire output when absent
}
```

`TextBlock::with_cache(cc: CacheControl)` marks a text block as a
prompt-caching boundary (feature `messages-caching`).

# Examples

```rust
use clauders::messages::{ContentBlock, TextBlock};
let block = ContentBlock::Text(TextBlock::new("hello"));
let j = serde_json::to_string(&block).unwrap();
assert_eq!(j, r#"{"type":"text","text":"hello"}"#);
```

Related: [Tool content blocks](/crates/clauders/messages/tools.md),
[CacheControl](/crates/clauders/types/caching.md),
[MessageRequest](/crates/clauders/messages/request.md),
[response Message](/crates/clauders/messages/response.md),
[agent ContentBlock](/crates/clauders/agent/content.md) (an independent,
tolerant-decode analogue used by the Agent SDK).

# Citations

1. `crates/clauders/src/messages/content.rs`
