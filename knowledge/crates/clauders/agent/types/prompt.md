---
type: Rust Newtype
title: clauders::agent::types::prompt::Prompt
description: Prompt — the text of one user turn sent to the agent; accepts plain UTF-8 text via From<&str>/From<String> so call sites can pass either through impl Into<Prompt>.
tags: [rust, sdk, agent, newtype, prompt]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/types/prompt.rs
---

Richer structured prompts are a future additive change; today `Prompt`
wraps plain text only.

# Schema

```rust
pub struct Prompt(String);
impl Prompt {
    pub fn new(text: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
    pub fn into_inner(self) -> String;
}
impl From<&str> for Prompt { ... }
impl From<String> for Prompt { ... }
```

# Examples

```rust
use clauders::agent::Prompt;
let p: Prompt = "hello".into();
assert_eq!(p.as_str(), "hello");
```

Related: [Client::query / query() free function](/crates/clauders/agent/client.md)
(both take `impl Into<Prompt>`), [Runtime::run](/crates/clauders/agent/runtime.md),
[cli::runtime user_message_frame](/crates/clauders/agent/cli/runtime.md)
(wraps `Prompt::as_str()` as the outbound user-message frame).

# Citations

1. `crates/clauders/src/agent/types/prompt.rs`
