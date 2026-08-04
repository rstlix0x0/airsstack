# Tutorial: your first Messages API call

By the end of this you will have sent a request to Claude from Rust, read the reply, streamed the
same request token by token, and completed a tool round-trip. Five steps, each one runnable.

This is a lesson, not a manual. It shows one path and does not stop to justify it. For the reasoning
see [explanation.md](explanation.md); for a specific recipe see [how-to.md](how-to.md).

## Before you start

An Anthropic API key in your environment:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

The crate and an async runtime:

```toml
[dependencies]
clauders = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Step 1 — build a client

One import line covers most call sites.

```rust
use clauders::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    println!("client ready");
    Ok(())
}
```

Run it. It prints `client ready` and exits.

`ApiKey::new` is where the key is validated — once you hold an `ApiKey`, it is a well-formed key. And
`build()` only exists after `api_key` has been called: delete that line and the program does not
compile, rather than failing at runtime.

## Step 2 — send a request

Build the request, send it, print the reply.

```rust
use clauders::messages::ContentBlock;
use clauders::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_5())
        .max_tokens(MaxTokens::new(1024))
        .add_user_text("Say hi.")
        .build();

    let msg = client.messages().create(req).await?;

    for block in &msg.content {
        if let ContentBlock::Text(t) = block {
            println!("{}", t.text);
        }
    }

    Ok(())
}
```

Claude's reply prints.

`MessageRequest::builder()` follows the same rule as the client builder: `model` and `max_tokens` are
required, and `build()` does not exist until both are set.

## Step 3 — read the rest of the response

The text is not the only thing that came back.

```rust
    println!("stop_reason: {:?}", msg.stop_reason);
    println!(
        "usage: input={} output={}",
        msg.usage.input_tokens, msg.usage.output_tokens
    );
```

`stop_reason` tells you why generation ended — `EndTurn` for a complete answer, `MaxTokens` if it hit
your ceiling, `ToolUse` if it wants to call a tool. `usage` is what you were billed for.

## Step 4 — stream the same request

`create` waits for the whole response. `stream` gives you fragments as they arrive.

```rust
use std::pin::Pin;

use clauders::messages::{ContentDelta, StreamEvent};
use clauders::prelude::*;
use futures_core::Stream as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_5())
        .max_tokens(MaxTokens::new(1024))
        .add_user_text("Count from one to five.")
        .build();

    let mut stream = client.messages().stream(req).await?;

    loop {
        let next = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        match next {
            None => break,
            Some(Ok(StreamEvent::ContentBlockDelta {
                delta: ContentDelta::TextDelta { text },
                ..
            })) => {
                use std::io::Write as _;
                print!("{text}");
                std::io::stdout().flush()?;
            }
            Some(Ok(_)) => {}
            Some(Err(e)) => return Err(e.into()),
        }
    }

    println!();
    Ok(())
}
```

The numbers appear one fragment at a time instead of all at once.

Add `futures-core = "0.3"` to your dependencies for the `Stream` trait.

## Step 5 — give Claude a tool

The last step has two turns: Claude asks to call your tool, you answer, Claude replies.

```rust
use clauders::messages::tools::{Tool, ToolChoice, ToolResultBlock};
use clauders::messages::{ContentBlock, ContentBlockParam, MessageContent, Role};
use clauders::prelude::*;
use clauders::types::ToolName;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(1024);

    let tool = Tool {
        name: ToolName::new("get_weather")?,
        description: "Look up the current weather for a city.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"]
        }),
        cache_control: None,
        strict: None,
        eager_input_streaming: None,
    };

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_5())
        .max_tokens(max_tokens)
        .add_user_text("What is the weather in Paris?")
        .tools([tool.clone()])
        .tool_choice(ToolChoice::Auto)
        .build();

    let assistant_msg = client.messages().create(req).await?;

    let Some(tu) = assistant_msg.content.iter().find_map(|b| match b {
        ContentBlock::ToolUse(tu) => Some(tu.clone()),
        _ => None,
    }) else {
        println!("Model did not call the tool.");
        return Ok(());
    };

    println!("Tool called: {} with input: {}", tu.name.as_str(), tu.input);

    // Your code would look the weather up here.
    let result_body =
        serde_json::json!({"temperature": "18°C", "condition": "partly cloudy"}).to_string();
    let tool_result = ToolResultBlock::text(tu.id.clone(), result_body);

    let follow_up = MessageRequest::builder()
        .model(ModelId::claude_sonnet_5())
        .max_tokens(max_tokens)
        .add_message(
            Role::User,
            MessageContent::Text("What is the weather in Paris?".into()),
        )
        .add_message(
            Role::Assistant,
            MessageContent::Blocks(vec![ContentBlockParam::ToolUse(tu)]),
        )
        .add_message(
            Role::User,
            MessageContent::Blocks(vec![ContentBlockParam::ToolResult(tool_result)]),
        )
        .tools([tool])
        .build();

    let final_msg = client.messages().create(follow_up).await?;
    for block in &final_msg.content {
        if let ContentBlock::Text(t) = block {
            println!("{}", t.text);
        }
    }

    Ok(())
}
```

Claude answers using the weather you supplied.

Notice the follow-up turn replays the whole conversation — the original question, the assistant's
tool call, and your result. The API is stateless; every request carries its own history.

Notice too that the tool-use block moved from `ContentBlock::ToolUse` (what came back) to
`ContentBlockParam::ToolUse` (what you send). Those are two different enums, on purpose.

## What you now know

You can build a client, send and stream requests, read usage and stop reasons, and run a tool
round-trip. That is the core of every program built on this client.

Next:

- **[how-to.md](how-to.md)** — recipes for specific goals, indexed against 11 runnable examples.
- **[explanation.md](explanation.md)** — why the builders are type-state, why request and response
  blocks are separate types, and what the streaming accumulator does.
- **[feature-parity.md](feature-parity.md)** — what matches the official Anthropic SDKs, and what
  does not.
