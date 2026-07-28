# 03 — Tools

A single tool (function-calling) round-trip: the model asks to call a tool, the
program returns a result, and the model produces a final answer.

For the multi-turn version that keeps running tools until the model is done, see
`07_agentic_tool_loop`.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 03_tools
```

## What it shows

**Declare a tool** with a JSON Schema for its input:

```rust
let tool = Tool {
    name: ToolName::new("get_weather")?,
    description: "Look up the current weather for a city.".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": { "city": { "type": "string" } },
        "required": ["city"]
    }),
    cache_control: None, strict: None, eager_input_streaming: None,
};

let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(1024))
    .add_user_text("What is the weather in Paris?")
    .tools([tool.clone()])
    .tool_choice(ToolChoice::Auto)
    .build();
```

**Read the tool call** from the assistant's `ContentBlock::ToolUse`:

```rust
let tu = assistant_msg.content.iter().find_map(|b| match b {
    ContentBlock::ToolUse(tu) => Some(tu.clone()),
    _ => None,
});
```

**Return the result** in a follow-up turn. The assistant turn is echoed back with
`ContentBlockParam::try_from_response` (an assistant `ContentBlock` is a
response-only type and must be converted before it can be re-sent), and the tool
output goes in a user turn as a `ToolResultBlock`:

```rust
let tool_result = ToolResultBlock::text(tu.id.clone(), result_json);

let follow_up = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(1024))
    .add_message(Role::User, MessageContent::Text("What is the weather in Paris?".into()))
    .add_message(Role::Assistant,
        MessageContent::Blocks(ContentBlockParam::try_from_response(assistant_msg.content.clone())?))
    .add_message(Role::User,
        MessageContent::Blocks(vec![ContentBlockParam::ToolResult(tool_result)]))
    .tools([tool])
    .tool_choice(ToolChoice::Auto)
    .build();
```
