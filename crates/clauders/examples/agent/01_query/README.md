# 01 — Query

The smallest Agent SDK program: one prompt, one session, print what comes back.

## Run

```text
cargo run -p clauders --example agent_01_query
```

No API key. The Agent SDK spawns the `claude` binary and talks to it over a pipe,
so it uses whatever credentials that binary already has.

## What it shows

```rust
use clauders::agent::{ContentBlock, Message, Options, query};
use futures_util::StreamExt;

let mut stream = query("Say hi in one short sentence.", Options::default()).await?;
while let Some(frame) = stream.next().await {
    match frame? { /* … */ }
}
```

- `query` connects, sends the prompt, and returns the stream in one call. The
  stream owns the session: when it is dropped, the subprocess is torn down.
- `Options::default()` is a complete, valid configuration — model, tools, and
  permission mode all come from the binary's own defaults. Example 02 fills it in.
- The stream item is `Result<Message, AgentError>`, so a per-frame decode failure
  is reported without ending the stream.

## Reading the frames

`Message` is an exhaustive enum. The two that matter here:

```rust
Message::Assistant(assistant) => {
    for block in &assistant.content {
        if let ContentBlock::Text { text } = block {
            println!("{text}");
        }
    }
}
Message::Result(result) => {
    println!("{}", result.session_id.as_str());
    println!("{}", result.num_turns);
}
```

`AssistantMessage::content` is a `Vec<ContentBlock>` — text, thinking, tool calls,
and tool results all arrive as blocks of one turn. `ContentBlock` is
`#[non_exhaustive]` and has an `Unknown` arm, so a block kind this release does not
model does not cost you the rest of the message.

`Message::Result` is the terminal frame: exactly one per turn. It carries the
session id (which example 13 uses to resume), the turn count, the cost, and the
token usage.

## Forward compatibility

```rust
Message::Other(raw) => println!("(unmodelled frame: {raw})"),
```

A frame whose `type` this SDK release does not know lands in `Message::Other` with
its JSON intact, rather than failing the turn. A newer `claude` binary can add
frame kinds without breaking this program.
