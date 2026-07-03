---
type: Rust Module
title: clauders::agent::types
description: Strongly-typed primitives specific to the Agent SDK — MCP server config/status, Prompt, and SessionId.
tags: [rust, sdk, agent, types]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/types/mod.rs
---

# Schema

| Submodule | Concept |
| --- | --- |
| `mcp` | [McpServerConfig / McpStatus / ServerStatus](/crates/clauders/agent/types/mcp.md) |
| `prompt` | [Prompt](/crates/clauders/agent/types/prompt.md) |
| `session_id` | [SessionId](/crates/clauders/agent/types/session-id.md) |

Distinct from the crate-root [types module](/crates/clauders/types/api-key.md)
(shared across the Messages/Models SDK) — these types are agent-only and
have no dependency on the `messages` feature.

# Citations

1. `crates/clauders/src/agent/types/mod.rs`
