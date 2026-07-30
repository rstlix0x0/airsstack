# 01 — Hello

The smallest possible request: send one user message, print the reply, the stop
reason, and the token usage.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 01_hello
```

## What it shows

Building a client, sending a non-streaming request, and reading the response.

```rust
let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
let client = Client::builder()?.api_key(api_key).build()?;

let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(1024))
    .add_user_text("Say hi.")
    .build();

let msg = client.messages().create(req).await?;
```

- `ApiKey::new` validates the key at construction — a bad key is an error here,
  not a surprise at request time.
- `MessageRequest::builder()` requires `model` and `max_tokens`; the program does
  not compile without them.
- `create(req)` performs `POST /v1/messages` and returns a decoded `Message`.

## Reading the response

`Message.content` is a `Vec<ContentBlock>`. A plain text reply arrives as one or
more `ContentBlock::Text` blocks:

```rust
for block in &msg.content {
    if let ContentBlock::Text(t) = block {
        println!("{}", t.text);
    }
}
println!("stop_reason: {:?}", msg.stop_reason);          // Option<StopReason>
println!("usage: input={} output={}",
    msg.usage.input_tokens, msg.usage.output_tokens);
```
