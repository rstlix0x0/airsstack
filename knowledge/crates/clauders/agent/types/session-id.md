---
type: Rust Newtype
title: clauders::agent::types::session_id::SessionId
description: SessionId — an opaque session identifier minted server-side by the claude binary and echoed back verbatim on control requests; the SDK does not validate or interpret its contents.
tags: [rust, sdk, agent, newtype, session]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/types/session_id.rs
---

# Schema

```rust
#[serde(transparent)]
pub struct SessionId(String);
impl SessionId {
    pub fn new(s: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl std::fmt::Display for SessionId { ... }
```

Unlike the crate-root [ID newtype family](/crates/clauders/types/ids.md),
`SessionId::new` is infallible — no non-empty validation, since the value
is always server-assigned and merely echoed.

# Examples

```rust
use clauders::agent::SessionId;
let id = SessionId::new("sess_abc123");
assert_eq!(id.as_str(), "sess_abc123");
```

Related: [ResultMessage::session_id](/crates/clauders/agent/message.md),
[MockRuntime](/crates/clauders/agent/mock.md) (test fixtures construct
`ResultMessage` with a `SessionId`).

# Citations

1. `crates/clauders/src/agent/types/session_id.rs`
