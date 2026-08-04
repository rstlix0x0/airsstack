# Tutorial: your first agent session

By the end of this you will have run an agent from Rust, read its output frame by frame, configured
it, held a two-turn conversation, and interrupted it mid-thought. Six steps, each one runnable.

This is a lesson, not a manual. It shows one path through and does not stop to justify it. When you
want the reasoning, read [explanation.md](explanation.md); when you want a specific recipe, read
[how-to.md](how-to.md).

## Before you start

You need a `claude` binary, version 2.0.0 or newer, on your `PATH`.

```bash
claude --version
```

You do **not** need `ANTHROPIC_API_KEY`. The Agent SDK never calls the HTTP API — it spawns the
`claude` binary and talks to it over a pipe, reusing whatever credentials that binary already has. If
this works in your shell, you are ready:

```bash
claude -p "hi"
```

Add the crate and an async runtime to a project:

```toml
[dependencies]
clauders = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"
```

## Step 1 — send a prompt

The smallest complete program. `query` takes a prompt and an `Options`, spawns the binary, and hands
back a stream.

```rust
use clauders::agent::{Message, Options, query};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = query("Say hi in one short sentence.", Options::default()).await?;

    while let Some(frame) = stream.next().await {
        println!("{:?}", frame?);
    }

    Ok(())
}
```

Run it. You will see several frames scroll past — a `System` frame first, then one or more
`Assistant` frames, then a `Result`. That is the whole shape of a turn: some output, then exactly one
terminal frame.

## Step 2 — read the frames you care about

Printing `{:?}` proved the pipe works. Now pull the text out.

`Message` is an enum with one variant per frame kind. Match on it and the compiler will not let you
drop a case silently.

```rust
use clauders::agent::{ContentBlock, Message, Options, query};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = query("Say hi in one short sentence.", Options::default()).await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => {
                for block in &assistant.content {
                    if let ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
            Message::Result(result) => {
                println!("---");
                println!("session:  {}", result.session_id.as_str());
                println!("turns:    {}", result.num_turns);
                if let Some(cost) = result.total_cost_usd {
                    println!("cost:     ${cost:.6}");
                }
            }
            Message::User(_) | Message::System(_) | Message::StreamEvent(_) => {}
            Message::Other(raw) => println!("(unmodelled frame: {raw})"),
        }
    }

    Ok(())
}
```

Now you get the sentence, then the session id, turn count and cost.

`Message::Other` is the catch-all. Leave it in — it is what stops a newer `claude` release from
breaking your program when it emits a frame kind this crate does not model yet.

## Step 3 — configure the session

`Options` is the one configuration argument. Build it and pass it where `Options::default()` was.

```rust
use clauders::agent::{Options, PermissionMode};

let options = Options::builder()
    .system_prompt("Answer in one short sentence. Never use a tool.")
    .permission_mode(PermissionMode::Plan)
    .max_turns(2)
    .build();

let mut stream = query("What is 2 + 2?", options).await?;
```

Three things changed: the agent got a system prompt, it was put in plan mode so it cannot execute
anything, and it was capped at two turns.

Everything on `Options` is fixed when the process spawns. That matters for the next step.

## Step 4 — hold a conversation

`query` connects, sends, and tears down. For a second turn that remembers the first, keep the session
alive with a `Client`.

```rust
use clauders::agent::{Client, ContentBlock, Message, Options, PermissionMode};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::builder()
        .system_prompt("Answer in one short sentence. Never use a tool.")
        .permission_mode(PermissionMode::Plan)
        .max_turns(2)
        .build();

    let client = Client::connect(options).await?;

    for prompt in [
        "Pick a number between 1 and 10 and tell me what it is.",
        "Double the number you just picked.",
    ] {
        println!("> {prompt}");
        let mut stream = client.query(prompt).await?;
        while let Some(frame) = stream.next().await {
            if let Message::Assistant(assistant) = frame? {
                for block in &assistant.content {
                    if let ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
        }
    }

    Ok(())
}
```

The second answer refers to the first. One subprocess served both turns.

Dropping the client tears the subprocess down.

## Step 5 — ask the session about itself

A connected `Client` can answer questions the one-shot `query` cannot. Some cost nothing — they read
the handshake response the SDK already holds.

Add this after `Client::connect`, before the loop:

```rust
println!("advertised models: {}", client.supported_models().len());
println!("slash commands:    {}", client.supported_commands().len());
```

No round-trip happens for those two. This one does — it asks the running session:

```rust
let context = client.get_context_usage().await?;
println!(
    "context: {} / {} tokens ({}%) on {}",
    context.total_tokens, context.max_tokens, context.percentage, context.model
);
```

## Step 6 — interrupt a turn

Live control is the reason to hold a `Client`. Start a long turn, let it run briefly, then stop it.

```rust
use std::time::Duration;

let client = Client::connect(Options::default()).await?;
let mut stream = client.query("Count slowly from 1 to 500.").await?;

tokio::time::sleep(Duration::from_secs(2)).await;
let receipt = client.interrupt().await?;
println!("interrupted; receipt: {receipt:?}");

while let Some(frame) = stream.next().await {
    if let Message::Result(result) = frame? {
        println!("stopped after {} turns", result.num_turns);
    }
}
```

The turn ends early and you still get a `Result` frame. The receipt reports which queued items
remained, when the binary says so.

## What you now know

You can spawn an agent, read its frames, configure it, keep a session across turns, query its state,
and interrupt it. That covers the shape of every program built on this crate.

Next:

- **[how-to.md](how-to.md)** — recipes for specific goals, indexed against 14 runnable examples.
- **[explanation.md](explanation.md)** — why it drives a subprocess, how the four internal layers
  divide, and why `Message::Other` exists.
- **[feature-parity.md](feature-parity.md)** — what matches the official Python and TypeScript Agent
  SDKs, and what does not.
