---
type: Rust Module
title: clauders::messages::tools
description: Tool (function-calling) types for the Messages API — Tool definitions, ToolChoice policy, and ToolUseBlock/ToolResultBlock content shapes.
tags: [rust, sdk, anthropic, messages-api, tools, function-calling]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/tools.rs
---

Gated behind `messages-tools` so tool-calling types are only compiled when
enabled, keeping the base messages surface free of tool-specific
dependencies. Registering `ToolUse`/`ToolResult` inside
[ContentBlock](/crates/clauders/messages/content.md) happens under the same
feature gate, in `content.rs`.

# Schema

```rust
pub struct Tool {
    pub name: ToolName,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub cache_control: Option<CacheControl>,  // feature messages-caching
    pub strict: Option<bool>,                 // features messages-tools + messages-structured-outputs
}

#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice { Auto, Any, Tool { name: ToolName }, None }

pub struct ToolUseBlock {
    pub id: ToolUseId,
    pub name: ToolName,
    pub input: serde_json::Value,
    pub cache_control: Option<CacheControl>, // feature messages-caching
}

pub struct ToolResultBlock { /* tool_use_id, content: ToolResultContent, is_error */ }
pub enum ToolResultContent { /* text or block array */ }
```

# Examples

```rust
use clauders::messages::tools::Tool;
use clauders::types::ToolName;
let tool = Tool {
    name: ToolName::new("get_weather").unwrap(),
    description: "Retrieve current weather for a city.".into(),
    input_schema: serde_json::json!({"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]}),
    cache_control: None,
    strict: None,
};
```

Related: [MessageRequestBuilder::tools/tool_choice](/crates/clauders/messages/request.md),
[ContentBlock](/crates/clauders/messages/content.md),
[ToolName / ToolUseId](/crates/clauders/types/ids.md),
[StopReason::ToolUse](/crates/clauders/messages/response.md).

# Citations

1. `crates/clauders/src/messages/tools.rs`
