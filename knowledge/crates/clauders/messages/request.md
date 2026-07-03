---
type: Rust Module
title: clauders::messages::request
description: MessageRequest and its type-state builder — the wire-format request body for POST /v1/messages, enforcing model and max_tokens at compile time.
tags: [rust, sdk, anthropic, messages-api, request, type-state]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/request.rs
---

Construct via `MessageRequest::builder()`, which uses the same type-state
pattern as [ClientBuilder](/crates/clauders/builder.md) — independently
scoped so the `Missing`/`Present` marker names do not collide — to make
`build()` uncallable until both `model` and `max_tokens` are supplied.

# Schema

```rust
pub enum Role { User, Assistant }              // lowercase on the wire

pub enum MessageContent {                       // untagged: string or block array
    Text(String),
    Blocks(Vec<ContentBlock>),
}

pub struct InputMessage { pub role: Role, pub content: MessageContent }

pub struct Metadata { pub user_id: Option<UserId> }

pub struct MessageRequest {
    pub model: ModelId,
    pub max_tokens: MaxTokens,
    pub messages: Vec<InputMessage>,
    pub system: Option<SystemPrompt>,
    pub temperature: Option<Temperature>,
    pub top_p: Option<TopP>,
    pub top_k: Option<TopK>,
    pub stop_sequences: Vec<StopSequence>,
    pub metadata: Option<Metadata>,
    pub tools: Vec<tools::Tool>,                       // feature messages-tools
    pub tool_choice: Option<tools::ToolChoice>,         // feature messages-tools
    pub output_config: Option<structured_outputs::OutputConfig>, // feature messages-structured-outputs
    pub(crate) stream: bool,                            // managed by MessagesResource
}
```

`MessageRequestBuilder<M, Mt>` setters: `model`, `max_tokens` (the two
type-state transitions), `add_user_text`, `add_assistant_text`,
`add_message`, `system`, `temperature`, `top_p`, `top_k`, `stop_sequences`,
`metadata`, `tools`, `tool_choice`, `output_config`. `build()` is only
defined on `MessageRequestBuilder<Present, Present>`.

# Examples

```rust
use clauders::messages::MessageRequest;
use clauders::types::{MaxTokens, ModelId};

let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(1024).unwrap())
    .add_user_text("Hello, Claude")
    .build();
assert_eq!(req.model.as_str(), "claude-sonnet-4-5");
```

Related: [MessagesResource::create/stream](/crates/clauders/messages/resource.md),
[ContentBlock](/crates/clauders/messages/content.md),
[Tool / ToolChoice](/crates/clauders/messages/tools.md),
[OutputConfig](/crates/clauders/messages/structured-outputs.md),
[numeric newtypes](/crates/clauders/types/numeric.md),
[SystemPrompt](/crates/clauders/types/system.md).

# Citations

1. `crates/clauders/src/messages/request.rs`
