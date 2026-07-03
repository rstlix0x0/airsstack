---
type: Rust Module
title: clauders::messages::token_counting
description: TokenCount response type and the CountTokensBody serialization projection backing POST /v1/messages/count_tokens.
tags: [rust, sdk, anthropic, messages-api, token-counting]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/token_counting.rs
---

Gated behind `messages-token-counting` (depends on `messages`, not enabled
by default). HTTP dispatch lives in
[MessagesResource::count_tokens](/crates/clauders/messages/resource.md);
response error decoding is shared crate-wide (`wire_helpers`).

# Schema

```rust
pub struct TokenCount { pub input_tokens: u32 }

pub(crate) struct CountTokensBody<'a> {
    pub(crate) model: &'a ModelId,
    pub(crate) messages: &'a [InputMessage],
    pub(crate) system: Option<&'a SystemPrompt>,
    pub(crate) tools: &'a [tools::Tool],              // feature messages-tools
    pub(crate) tool_choice: Option<&'a tools::ToolChoice>, // feature messages-tools
}
```

`CountTokensBody::from_request(&MessageRequest)` builds the projection,
intentionally omitting `max_tokens`, `temperature`, `top_p`, `top_k`,
`stop_sequences`, `metadata`, and `stream` — fields the count-tokens
endpoint rejects.

# Examples

```rust
use clauders::messages::token_counting::TokenCount;
let tc: TokenCount = serde_json::from_str(r#"{"input_tokens":42}"#).unwrap();
assert_eq!(tc.input_tokens, 42);
```

Related: [MessagesResource::count_tokens](/crates/clauders/messages/resource.md),
[MessageRequest](/crates/clauders/messages/request.md).

# Citations

1. `crates/clauders/src/messages/token_counting.rs`
