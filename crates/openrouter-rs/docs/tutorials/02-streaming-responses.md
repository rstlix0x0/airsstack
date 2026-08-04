# Tutorial 2 — Streaming a response

[Tutorial 1](01-first-completion.md) waited for the whole completion before
printing anything. Here you will print each token as it arrives, and learn the
one rule that makes `ChatStream` different from an ordinary `Stream`.

## Step 1 — Add a stream extension trait

`ChatStream` implements `futures_core::Stream`, which gives you `poll_next` and
nothing else. To write `while let Some(chunk) = stream.next().await` you need
`StreamExt`, which lives in `futures-util`:

```toml
[dependencies]
futures-util = "0.3"
```

```rust
use futures_util::StreamExt;
```

## Step 2 — Build the same request

Nothing changes. You do **not** set a `stream` flag — the resource layer flips
it for you when you call `stream()`, and the field is not part of the builder
surface.

```rust
let req = ChatRequest::builder()
    .model(ModelId::custom("openai/gpt-4o-mini")?)
    .messages(vec![Message::user("Count from one to five.")])
    .build();
```

## Step 3 — Call `stream` instead of `send`

```rust
let mut stream = client.chat().stream(req).await?;
```

The `await` here resolves once the response **headers** arrive, not once the
body is complete. If the server answered with a non-2xx status, this is where
you find out: the status is checked eagerly and the error body is decoded before
any stream handle is handed back. A `ChatStream` in your hands means the request
succeeded.

## Step 4 — Drain it

Each item is a `Result<StreamChunk, Error>`. A chunk carries a `delta` — a
*fragment* of the message, not a whole one — so you append rather than replace.

```rust
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(choice) = chunk.choices.first() {
        if let Some(text) = &choice.delta.content {
            print!("{text}");
            std::io::stdout().flush()?;
        }
    }
}
println!();
```

`flush` matters: `print!` without a newline sits in the line buffer, and the
whole point of streaming is watching it appear.

## The rule: the stream is terminal on error

Once `ChatStream` yields an `Err`, the next poll returns `None`. It does not
recover, and it does not yield further chunks. Three things can end a stream:

| Ending | What you observe |
|---|---|
| `data: [DONE]` | `None`. Clean completion. |
| Mid-stream error event, or transport interruption | one `Err(Error::Stream(msg))`, then `None` |
| A `data:` line that is not decodable JSON | one `Err(Error::Serde { .. })`, then `None` |

Because of this, the `chunk?` above is not just convenient — it is correct. Any
loop that swallows the error and keeps polling terminates immediately anyway.

## Where the usage numbers are

Only the final chunk carries `usage`. If you want token counts from a streaming
call, capture them as you go:

```rust
let mut total = None;
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(u) = chunk.usage {
        total = Some(u.total_tokens);
    }
}
```

## The whole program

```rust
use std::io::Write;

use futures_util::StreamExt;
use openrouter_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    let req = ChatRequest::builder()
        .model(ModelId::custom("openai/gpt-4o-mini")?)
        .messages(vec![Message::user("Count from one to five.")])
        .build();

    let mut stream = client.chat().stream(req).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(choice) = chunk.choices.first() {
            if let Some(text) = &choice.delta.content {
                print!("{text}");
                std::io::stdout().flush()?;
            }
        }
    }
    println!();

    Ok(())
}
```

This is `examples/02_streaming.rs` verbatim:

```bash
OPENROUTER_API_KEY=sk-... cargo run --example 02_streaming
```

## Next

You now have both response shapes. From here the how-to guides are
independent — [call tools](../how-to/call-tools.md),
[request structured outputs](../how-to/request-structured-outputs.md), or
[steer provider routing](../how-to/steer-provider-routing.md). For the details
of what a chunk contains, see [reference/streaming.md](../reference/streaming.md).
