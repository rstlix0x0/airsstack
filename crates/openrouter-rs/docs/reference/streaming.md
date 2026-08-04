# Reference — Streaming

Streaming uses the same endpoint with `stream: true` in the body and
`Accept: text/event-stream`. SSE framing is handled by `eventsource-stream`.

## Entry points

```rust
ChatResource::stream(&self, req: ChatRequest) -> Result<ChatStream, Error>
ChatResource::stream_cached(&self, req: ChatRequest, cache: ResponseCache)
    -> Result<Cached<ChatStream>, Error>
```

Both mutate a local copy of the request to set `stream = true`. Neither requires
you to touch that field; it has no builder method.

**The status is checked eagerly.** A non-2xx response is drained (subject to the
16 MiB cap) and decoded into an `Error` before any stream handle exists. Holding
a `ChatStream` means the request succeeded.

## `ChatStream`

```rust
impl Stream for ChatStream { type Item = Result<StreamChunk, Error>; }
```

`#[must_use = "streams do nothing unless polled"]`. `Debug` prints only the
`terminated` flag. To use `.next()`, bring a stream extension trait into scope —
`futures_util::StreamExt`.

### Termination rules

`ChatStream` is **terminal on error**: after it yields an `Err`, the next poll
returns `None`, permanently.

| Event | Yielded | Then |
|---|---|---|
| `data: [DONE]` | — | `None` |
| Underlying body stream ends | — | `None` |
| SSE framing error / transport interruption | `Err(Error::Stream(msg))` | `None` |
| Chunk carries a mid-stream `error` object | `Err(Error::Stream(message))` | `None` |
| `data:` payload is not decodable JSON | `Err(Error::Serde { context: "StreamChunk", .. })` | `None` |
| Anything else | `Ok(StreamChunk)` | continues |

The `[DONE]` sentinel is matched against the exact string `[DONE]`.

## `StreamChunk`

| Field | Type | Notes |
|---|---|---|
| `id` | `String` | stable across the chunks of one response |
| `object` | `String` | `"chat.completion.chunk"` |
| `created` | `u64` | Unix seconds |
| `model` | `String` | as echoed by the server |
| `choices` | `Vec<ChunkChoice>` | |
| `usage` | `Option<Usage>` | **final chunk only** |

There is a seventh field, `error: Option<ChunkError>`, which is crate-private.
The stream driver converts a chunk carrying it into a terminal
`Error::Stream(message)`, so a chunk handed to you always has it unset. Only
`message` is modelled; the server's `code` has an inconsistent wire type across
providers and is not surfaced.

Unknown top-level fields — the gateway's `provider` string, for instance — are
ignored.

## `ChunkChoice`

| Field | Type | Notes |
|---|---|---|
| `index` | `u32` | |
| `delta` | `ChunkDelta` | |
| `finish_reason` | `Option<FinishReason>` | `None` until the final chunk for that choice |

## `ChunkDelta`

| Field | Type | Notes |
|---|---|---|
| `role` | `Option<Role>` | typically the first chunk of a choice only |
| `content` | `Option<String>` | the fragment to **append** |

Both are absent on a chunk that only updates `finish_reason`. `ChunkDelta`
derives `Default`.

**There is no `tool_calls` field on the delta.** Streaming cannot observe an
incremental tool call; use `send` for tool-calling turns.

## Reassembling a message

```rust
use futures_util::StreamExt;

let mut text = String::new();
let mut finish = None;
let mut usage = None;

let mut stream = client.chat().stream(req).await?;
while let Some(item) = stream.next().await {
    let chunk = item?;                       // terminal on Err — do not swallow
    if let Some(c) = chunk.choices.first() {
        if let Some(frag) = &c.delta.content {
            text.push_str(frag);
        }
        if c.finish_reason.is_some() {
            finish = c.finish_reason;
        }
    }
    if chunk.usage.is_some() {
        usage = chunk.usage;
    }
}
```

## Body size

A streaming 2xx body is not drained into memory and is not subject to
`MAX_RESPONSE_BODY_BYTES`. That cap applies to the non-2xx error body on the
streaming paths, and to every response on the non-streaming paths.

## Where the pieces live

| Item | Module |
|---|---|
| `ChatStream` | `chat::stream` |
| `StreamChunk`, `ChunkChoice`, `ChunkDelta` | `chat::stream_chunk` |
| `stream` / `stream_cached` | `chat::resource` |

`ChatStream` and `StreamChunk` are exported from the crate root, the `chat`
module, and the prelude. `ChunkChoice` and `ChunkDelta` are exported from `chat`
only.
