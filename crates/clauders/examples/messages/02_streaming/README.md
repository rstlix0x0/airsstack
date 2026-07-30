# 02 — Streaming

Stream the reply and print each text fragment as it arrives, instead of waiting
for the whole message.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 02_streaming
```

## What it shows

`stream()` returns a `MessageStream`, which implements `futures_core::Stream`.
Each item is a `Result<StreamEvent, Error>`. The text lands in
`ContentBlockDelta` events carrying a `ContentDelta::TextDelta`:

```rust
use clauders::messages::{ContentDelta, StreamEvent};
use futures_core::Stream as _;

let mut stream = client.messages().stream(req).await?;

loop {
    let next = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
    match next {
        None => break,
        Some(Ok(StreamEvent::ContentBlockDelta {
            delta: ContentDelta::TextDelta { text }, ..
        })) => print!("{text}"),
        Some(Ok(_)) => {}          // message_start, content_block_start, ping, ...
        Some(Err(e)) => return Err(e.into()),
    }
}
```

## Notes

- The stream also emits `MessageStart`, `ContentBlockStart`, `ContentBlockStop`,
  `MessageDelta`, and `MessageStop`; this example ignores all but the text
  deltas. Match more variants to track usage or stop reason mid-stream.
- To rebuild the full final `Message` from the event stream, use
  `MessageAccumulator` (see the `clauders::messages::accumulator` module).
