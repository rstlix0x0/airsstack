# 07 — Agentic tool loop

The multi-turn version of `03_tools`: keep sending the conversation, and while
the model stops on `tool_use`, run every tool call, append the results, and send
again — until the model returns a normal end-of-turn answer. Asking about two
cities forces more than one tool call.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 07_agentic_tool_loop
```

## What it shows

**Hold the running conversation** as a growing list of turns, and rebuild the
request from it each iteration:

```rust
let mut history: Vec<(Role, MessageContent)> = vec![(
    Role::User,
    MessageContent::Text("What is the weather in Paris and Tokyo?".into()),
)];

for turn in 1..=6 {                       // cap so it cannot spin forever
    let mut builder = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(1024));
    for (role, content) in &history {
        builder = builder.add_message(*role, content.clone());
    }
    let msg = client.messages()
        .create(builder.tools([tool.clone()]).tool_choice(ToolChoice::Auto).build())
        .await?;
    // ...
}
```

**Record the assistant turn**, then **stop unless it wants a tool**:

```rust
history.push((Role::Assistant,
    MessageContent::Blocks(ContentBlockParam::try_from_response(msg.content.clone())?)));

if msg.stop_reason != Some(StopReason::ToolUse) {
    break;
}
```

**Run every tool call** and feed the results back as one user turn:

```rust
let mut results = Vec::new();
for block in &msg.content {
    if let ContentBlock::ToolUse(tu) = block {
        let output = weather_for(&tu.input);   // your real tool here
        results.push(ContentBlockParam::ToolResult(
            ToolResultBlock::text(tu.id.clone(), output)));
    }
}
history.push((Role::User, MessageContent::Blocks(results)));
```

## Notes

- The `stop_reason == ToolUse` check is the loop condition — the same signal a
  real agent runtime keys off.
- The tool is faked in-process (`weather_for`); replace it with a real call.
- The turn cap (`1..=6`) is a safety net against a model that never settles.
