# How to call tools

A tool call is a three-message dance: you ask, the model asks you to run a
function, you answer, the model replies. This guide walks the whole round trip.

## 1. Define the tool

`FunctionName` enforces the API's charset (`[A-Za-z0-9_-]`, 1–64 characters) at
construction. `parameters` is a raw JSON Schema value — the crate does not
generate it for you.

```rust
use openrouter_rs::prelude::*;
use serde_json::json;

let weather = Tool::function(FunctionDef {
    name: FunctionName::new("get_weather")?,
    description: Some("Get the current temperature for a city.".into()),
    parameters: Some(json!({
        "type": "object",
        "properties": { "city": { "type": "string" } },
        "required": ["city"]
    })),
    strict: None,
});
```

`FunctionDef::new(name)` gives you the same thing with all three optionals set
to `None`, if a bare name is all you need.

## 2. Send the first turn

```rust
let first = ChatRequest::builder()
    .model(model.clone())
    .messages(vec![Message::user("What is the weather in Paris?")])
    .tools(vec![weather.clone()])
    .tool_choice(ToolChoice::Auto)
    .build();

let completion = client.chat().send(first).await?;
```

`ToolChoice` controls how hard you push:

| Variant | Wire form | Meaning |
|---|---|---|
| `ToolChoice::None` | `"none"` | Do not call a tool. |
| `ToolChoice::Auto` | `"auto"` | Model decides. |
| `ToolChoice::Required` | `"required"` | Must call *some* tool. |
| `ToolChoice::Function { name }` | `{"type":"function","function":{"name":"…"}}` | Must call this one. |

Omitting `.tool_choice(...)` entirely leaves the field off the wire, which also
lets the model decide.

## 3. Read the call

When the model decides to call a tool, `content` is `None`, `finish_reason` is
`FinishReason::ToolCalls`, and `tool_calls` carries the list.

```rust
let choice = completion.choices.first().ok_or("no choice returned")?;
let calls = choice
    .message
    .tool_calls
    .as_ref()
    .ok_or("model did not request a tool call")?;
let call = calls.first().ok_or("empty tool_calls")?;

println!("model called: {}({})", call.function.name, call.function.arguments);
```

**`arguments` is a raw JSON string, not a parsed object.** That is deliberate:
it is exactly what the server sent, byte for byte, so nothing is lost to a
re-serialisation round trip. Parse it yourself:

```rust
#[derive(serde::Deserialize)]
struct WeatherArgs { city: String }

let args: WeatherArgs = serde_json::from_str(&call.function.arguments)?;
```

## 4. Replay the assistant turn, then answer

The model needs to see its own tool-call message before it sees your result.
`Message::assistant_tool_calls` builds that replay message with no text content
— the `content` field is omitted from the wire entirely, which is what the API
expects.

```rust
let call_id = ToolCallId::new(call.id.as_str())?;

let second = ChatRequest::builder()
    .model(model)
    .messages(vec![
        Message::user("What is the weather in Paris?"),
        Message::assistant_tool_calls(calls.clone()),
        Message::tool_result(call_id, "18 degrees Celsius, clear skies."),
    ])
    .tools(vec![weather])
    .build();

let final_completion = client.chat().send(second).await?;
```

`Message::tool_result(id, content)` produces `role: "tool"` with the
`tool_call_id` set. The id must match the one the server issued; that is what
pairs your answer to its question.

Keep `.tools(...)` on the second turn. The model needs the definitions in scope
to interpret the exchange.

## 5. Read the final reply

```rust
if let Some(text) = final_completion
    .choices
    .first()
    .and_then(|c| c.message.content.as_deref())
{
    println!("final: {text}");
}
```

The complete program is `examples/03_tools.rs`:

```bash
OPENROUTER_API_KEY=sk-... cargo run --example 03_tools
```

## Handling several calls at once

`tool_calls` is a `Vec` and a model may request more than one function in a
single turn. Run them all, then send one `Message::tool_result` per call, each
carrying its own id, after a single `assistant_tool_calls` replay:

```rust
let mut messages = vec![
    Message::user(question),
    Message::assistant_tool_calls(calls.clone()),
];
for call in calls {
    let output = dispatch(&call.function.name, &call.function.arguments)?;
    messages.push(Message::tool_result(ToolCallId::new(call.id.as_str())?, output));
}
```

## Limitation: tool calls do not stream

`ChunkDelta` carries only `role` and `content`. Streamed chunks have no
`tool_calls` field, so a streaming request cannot observe an incremental tool
call. Use `send` for tool-calling turns.
