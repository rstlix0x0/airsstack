# 04 — Partial streaming

Print text as it is produced, watch the child's stderr, and see what happens to a
frame this SDK release does not model.

## Run

```text
cargo run -p clauders --example agent_04_partial_streaming
```

## What it shows

By default the stream carries whole messages: one `Message::Assistant` per model
turn, delivered when the turn is complete. One flag adds the token-level frames
underneath it:

```rust
let options = Options::builder()
    .include_partial_messages(true)
    .stderr(move |chunk: &str| { counter.fetch_add(chunk.len(), Ordering::Relaxed); })
    .build();
```

## Partial frames

```rust
Message::StreamEvent(event) => {
    if let Some(text) = delta_text(&event.event) {
        print!("{text}");
        std::io::stdout().flush()?;
    }
}

fn delta_text(event: &serde_json::Value) -> Option<&str> {
    let delta = event.get("delta")?;
    delta.get("text")
        .or_else(|| delta.get("thinking"))
        .and_then(serde_json::Value::as_str)
}
```

Printing **here**, rather than on the completed frame, is what makes the output
appear as the model produces it. That is the whole reason to ask for partials.

`StreamEvent::event` is an opaque `serde_json::Value` holding the Messages API's own
streaming event. It is not typed here because it is the *inner* API's wire shape
passed through by the binary; a caller reads the fields it cares about and ignores
the rest. A `content_block_delta` carries `delta.text` for text or `delta.thinking`
for extended thinking; every other event kind — block start and stop, message start
and delta — carries no text.

## Partials are a request, not a guarantee

`include_partial_messages(true)` asks the binary for these frames. A given binary
and run may send none, so an example that printed *only* deltas would print nothing
at all. The loop here handles both worlds:

- deltas arrived → they were already printed live, so the completed
  `Message::Assistant` is a duplicate and only its size is reported;
- no deltas arrived → the completed frame is the only copy, so it is printed in
  full, and the run says so.

The completed frame is the authoritative copy either way. A program that does not
care about latency can ignore partials entirely and use just that — which is what
every other example in this directory does.

## The stderr callback

```rust
.stderr(|chunk: &str| { /* … */ })
```

Invoked with one valid-UTF-8 chunk at a time as the child writes to stderr. Two
things to know:

- It **augments** capture rather than replacing it. The stderr tail is still kept
  and still appears in `AgentError::Cli` if the process dies.
- It runs on the runtime's reader task, so the closure must be `Send + Sync`. This
  example increments an `AtomicUsize` through an `Arc`; printing the chunk works
  just as well.

`max_buffer_size(NonZeroUsize)` is the related knob: it caps how many bytes are
buffered for a single stdout line before the SDK errors instead of growing without
bound. Unset means unbounded.

## Frames this release does not model

```rust
Message::Other(raw) => {
    let kind = raw.get("type").and_then(serde_json::Value::as_str).unwrap_or("<untyped>");
    println!("[unmodelled frame: {kind}]");
}
```

`Message::Other` is the forward-compatibility arm: any line whose `type` matches no
known frame is captured verbatim instead of failing the turn. The hook-lifecycle
frames from `include_hook_events(true)` (example 06) are the common case today.

## Timing

`ResultMessage` carries `ttft_ms`, `duration_ms`, and `duration_api_ms` when the
binary reports them — the time-to-first-token figure is the one worth comparing
against what you saw the partials do.
