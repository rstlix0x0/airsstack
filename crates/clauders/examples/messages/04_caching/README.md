# 04 — Prompt caching

Write a cache block on a first request, then read it back on a second — showing
the cache-usage counters change between calls.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 04_caching
```

## What it shows

A cache breakpoint is placed on a **system-prompt segment**. Both requests send
an identical system prefix, so the second call reads the block the first wrote:

```rust
use clauders::types::{CacheControl, SystemPrompt, SystemSegment};

let segment = SystemSegment::text(system_text.clone())
    .with_cache(CacheControl::ephemeral());

let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(64))
    .system(SystemPrompt::segments(vec![segment]))
    .add_user_text("Reply with the single word: ok.")
    .build();
```

**Read the cache counters** off `Usage`:

```rust
msg.usage.input_tokens;                  // uncached input this call
msg.usage.cache_creation_input_tokens;   // Option — tokens written to cache
msg.usage.cache_read_input_tokens;       // Option — tokens served from cache
msg.usage.total_input_tokens();          // sum of the three
```

Expect `cache_creation` > 0 on the first call and `cache_read` > 0 on the second.

## Notes

- The server only stores a cache block once the cached prefix exceeds a minimum
  length (~1024 tokens for Sonnet). The example repeats the crate README to clear
  that threshold; a real app would cache a genuinely large, stable prompt or
  document corpus.
- `CacheControl::ephemeral()` is the 5-minute tier. Cache breakpoints can also be
  set on document, image, tool, and text blocks via their `cache_control` field.
