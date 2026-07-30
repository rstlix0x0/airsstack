# 11 — Warm start and streaming input

Two ways to control *when* work starts: pay the startup cost early, and feed user
turns in as they arrive.

## Run

```text
cargo run -p clauders --example agent_11_streaming_input
```

## Warm start

```rust
let warm = Client::startup(options).await?;
// … the subprocess is already up and handshaken …
let mut session = warm.query("Name one Rust crate for async I/O.").await?;

while let Some(frame) = session.stream().next().await { /* … */ }
```

`Client::startup` spawns, handshakes, and hands back a `WarmQuery`. Use it when the
prompt is not known yet but the latency is: a UI that wants the session ready before
the user finishes typing.

`WarmQuery::query` **consumes** `self`, so calling it twice does not compile — the
"exactly one prompt" rule is a type-level guarantee rather than a runtime check. To
tear a warmed session down without ever querying it, call `WarmQuery::close()`.

The returned `WarmSession` exposes `stream()` for the turn and `interrupt()` for
cutting it short.

Note what warm start does *not* add: `Client::connect` already completes the
`initialize` round-trip eagerly, bounded by `control_request_timeout`. Warm start adds
only the single-shot handle on top, which is why there is no separate warm-start
timeout.

## Streaming input

```rust
let turns = futures_util::stream::iter([
    "Remember the number 7.".to_owned(),
    "Add 5 to it.".to_owned(),
    "What is the running total?".to_owned(),
])
.then(|text| async move {
    tokio::time::sleep(Duration::from_millis(400)).await;
    text
});

let mut stream = client.query(Prompt::stream(turns)).await?;
```

`Prompt` has two forms:

- `Prompt::Single(String)` — one user turn. `From<&str>` and `From<String>` both
  produce it, which is why every other example passes a bare string literal to
  `query`.
- `Prompt::stream(s)` — any `Stream<Item = String> + Send + 'static`, whose items are
  fed into the live turn as they arrive.

Streaming input is what you want when turns come from somewhere else: a UI, a socket,
a queue, another task. The example spaces them with a sleep to stand in for input that
genuinely arrives over time; in real use the stream is usually a channel receiver.

The agent's replies come back on one stream regardless, so the read side of the loop
is identical to every other example.
