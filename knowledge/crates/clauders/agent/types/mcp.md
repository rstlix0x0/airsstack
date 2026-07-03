---
type: Rust Module
title: clauders::agent::types::mcp
description: External MCP server configuration (opaque pass-through) and status types — McpServerConfig, ServerStatus, and the aggregate McpStatus returned by the mcp_status control request.
tags: [rust, sdk, agent, mcp]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/types/mcp.rs
---

In-process MCP tools are unimplemented; external MCP servers are forwarded
to the binary opaquely — the SDK carries the raw JSON config untouched, so
a newer binary's config shape needs no SDK change.

# Schema

```rust
pub struct McpServerConfig { name: String, config: serde_json::Value }
impl McpServerConfig {
    pub fn new(name: impl Into<String>, config: serde_json::Value) -> Self;
    pub fn name(&self) -> &str;
    pub const fn config(&self) -> &serde_json::Value;
}

pub struct ServerStatus { pub name: String, pub status: String } // e.g. "connected", "failed"

pub struct McpStatus { pub servers: Vec<ServerStatus> } // Default
```

# Examples

```rust
use clauders::agent::McpServerConfig;
let raw = serde_json::json!({"command": "node", "args": ["server.js"]});
let cfg = McpServerConfig::new("fs", raw.clone());
assert_eq!(cfg.name(), "fs");
assert_eq!(cfg.config(), &raw);
```

Related: [Options::mcp_servers](/crates/clauders/agent/options.md),
[Client::mcp_status](/crates/clauders/agent/client.md),
[cli::argv::build_argv](/crates/clauders/agent/cli/argv.md) (emits one
`--mcp-config` per server), [OutboundRequestBody::McpStatus](/crates/clauders/agent/protocol/frames.md).

# Citations

1. `crates/clauders/src/agent/types/mcp.rs`
