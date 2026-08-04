# Tutorial 1 — Your first completion

By the end of this tutorial you will have a Rust program that sends a chat
request through OpenRouter and prints the model's reply, the reason generation
stopped, and the token usage.

You need an OpenRouter API key. Export it before you run anything:

```bash
export OPENROUTER_API_KEY=sk-or-v1-...
```

## Step 1 — Add the dependencies

`openrouter-rs` is async and does not ship a runtime, so you also need `tokio`.

```toml
[dependencies]
openrouter-rs = { path = "../openrouter-rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Step 2 — Import the prelude

The crate has a single glob-import surface. It carries the client, the request
and response types, and the validated newtypes you need to build a request.

```rust
use openrouter_rs::prelude::*;
```

## Step 3 — Build a client

Two things happen here. `ApiKey::new` validates the key string and wraps it so
it cannot be printed by accident. `Client::builder()` constructs the default
`reqwest`-backed transport, which is why it returns a `Result`.

```rust
let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
let client = Client::builder()?.api_key(api_key).build()?;
```

Note the order: `build()` does not exist until `api_key` has been supplied. Try
deleting the `.api_key(...)` call and compiling — the error is
`no method named 'build' found`, not a runtime panic. That is the type-state
builder at work; [Type-state builders](../explanation/type-state-builders.md)
explains the mechanism.

## Step 4 — Build a request

`ChatRequest` has two required fields, `model` and `messages`, and the same
compile-time rule applies to both. You may set them in either order.

```rust
let req = ChatRequest::builder()
    .model(ModelId::custom("openai/gpt-4o-mini")?)
    .messages(vec![Message::user("Say hi in one word.")])
    .build();
```

`ChatRequest::builder().build()` is infallible — every value it holds was
already validated when you constructed it, so there is no error path left.

## Step 5 — Send it

`client.chat()` hands you a short-lived resource handle borrowing the client.
Create it at the call site; there is no reason to store it.

```rust
let completion = client.chat().send(req).await?;
```

## Step 6 — Read the response

`choices` is a `Vec`, and `content` is `Option<String>` because a tool-calling
turn returns `null` content. Handle both rather than indexing blindly.

```rust
if let Some(choice) = completion.choices.first() {
    if let Some(text) = &choice.message.content {
        println!("{text}");
    }
    println!("finish_reason: {:?}", choice.finish_reason);
}
if let Some(usage) = &completion.usage {
    println!(
        "usage: prompt={} completion={} total={}",
        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
    );
}
```

## The whole program

```rust
use openrouter_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("OPENROUTER_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    let req = ChatRequest::builder()
        .model(ModelId::custom("openai/gpt-4o-mini")?)
        .messages(vec![Message::user("Say hi in one word.")])
        .build();

    let completion = client.chat().send(req).await?;

    if let Some(choice) = completion.choices.first() {
        if let Some(text) = &choice.message.content {
            println!("{text}");
        }
        println!("finish_reason: {:?}", choice.finish_reason);
    }
    if let Some(usage) = &completion.usage {
        println!(
            "usage: prompt={} completion={} total={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}
```

This is `examples/01_chat.rs` verbatim. Run it from the crate directory:

```bash
OPENROUTER_API_KEY=sk-... cargo run --example 01_chat
```

## Why `Box<dyn std::error::Error>`

The `?` operator is doing three different conversions here: `VarError` from
`std::env::var`, `InvalidApiKey` from `ApiKey::new`, and `Error` from the SDK
call. The newtype rejection errors are **not** variants of the SDK's `Error`
type, so a function returning `Result<(), openrouter_rs::error::Error>` cannot
absorb them with `?`. Boxing is the shortest path in an example; a real
application defines its own error enum. See
[reference/errors.md](../reference/errors.md#what-error-does-not-absorb).

## Next

[Tutorial 2 — Streaming a response](02-streaming-responses.md) takes the same
request and switches it to Server-Sent Events.
