---
type: Rust Module
title: clauders::agent::runtime::api::cache::CachePolicy
description: CachePolicy and its breakpoint-placement functions — how ApiRuntime marks the stable system+tools prefix and, optionally, the rolling conversation as cacheable across the repeated per-turn sends of the tool loop.
tags: [rust, sdk, agent, runtime, messages-api, prompt-caching]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/api/cache.rs
---

Cached request tokens bill at a fraction of fresh tokens on a cache hit, at
the cost of a one-time write. Because [ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md)
re-sends the system prompt and tool catalog on every turn of its loop,
marking that stable prefix — and optionally the running conversation — as
cacheable turns repeat sends into cache reads.

# Schema

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CachePolicy {
    Off,                        // no breakpoints; byte-identical to uncached
    Prefix,                     // cache system prompt + full tool catalog only
    #[default]
    PrefixAndConversation,      // Prefix, plus a rolling conversation breakpoint
}

pub(super) fn apply_prefix(policy: CachePolicy, system: &mut Option<SystemPrompt>, tools: &mut [Tool]);
pub(super) fn apply_conversation(policy: CachePolicy, history: &mut [InputMessage]);
```

`apply_prefix`: a no-op under `Off`. Otherwise, the breakpoint goes on the
**last tool** when any tool is declared — the API caches everything up to
and including it, i.e. the system prompt and every tool in one write. With
no tools, the system prompt itself is rebuilt as a single cached
[`SystemSegment`](/crates/clauders/types/system.md) (`cache_system_prompt`
converts a `SystemPrompt::Text` or the last segment of a
`SystemPrompt::Segments` into a `CacheControl::ephemeral()`-marked
segment). With neither tools nor a system prompt, there is nothing stable
to cache and this is a no-op.

`apply_conversation`: only active under `PrefixAndConversation`. Scans
history from the end for the most recent block-form (`MessageContent::Blocks`)
turn and marks the last cacheable block within it — `Text`, `ToolUse`, and
`ToolResult` blocks all carry `cache_control`; `Thinking` blocks do not and
are skipped. The initial plain-text user turn (`MessageContent::Text`)
carries no per-block breakpoint and is left untouched. This is a *rolling*
breakpoint: it is recomputed and re-marked on every turn as the
conversation grows, always on the freshest cacheable content.

# Examples

```rust
use clauders::agent::runtime::api::CachePolicy;
assert_eq!(CachePolicy::default(), CachePolicy::PrefixAndConversation);
```

Related: [types::caching (CacheControl/CacheTtl)](/crates/clauders/types/caching.md),
[types::system (SystemPrompt/SystemSegment)](/crates/clauders/types/system.md),
[messages::content](/crates/clauders/messages/content.md),
[messages::tools (Tool)](/crates/clauders/messages/tools.md),
[ApiRuntime::build_request](/crates/clauders/agent/runtime/api/runtime.md)
(the sole caller of both functions, once per turn).

# Citations

1. `crates/clauders/src/agent/runtime/api/cache.rs`
