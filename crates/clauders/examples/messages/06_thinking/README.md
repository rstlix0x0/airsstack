# 06 — Extended thinking

Let the model reason before answering, then read the thinking and the final
answer as separate blocks.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 06_thinking
```

## What it shows

**Turn thinking on** with an explicit token budget:

```rust
use clauders::messages::ThinkingConfig;

let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(4096))
    .thinking(ThinkingConfig::enabled(1024))   // budget must be < max_tokens
    .add_user_text("A farmer has 17 sheep. All but 9 run away. How many are left? Think it through.")
    .build();
```

**Read the two block kinds** from the response — thinking and the answer arrive
separately:

```rust
for block in &msg.content {
    match block {
        ContentBlock::Thinking(t) => println!("thinking: {}", t.thinking),
        ContentBlock::Text(t)     => println!("answer: {}", t.text),
        _ => {}
    }
}
```

## Notes

- The budget must be **at least 1024** tokens and **strictly less than
  `max_tokens`**; the server rejects a budget that breaks either bound.
- Constructors: `ThinkingConfig::enabled(budget)`, `adaptive()` (model decides),
  `disabled()`, plus `*_with_display` variants that set `ThinkingDisplay`
  (`Summarized` — the default — or `Omitted`).
- Not every model supports extended thinking.
