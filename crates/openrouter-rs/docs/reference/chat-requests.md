# Reference — Chat requests

`POST /chat/completions`. Path constant `chat/completions`, joined onto the
configured base URL.

## `ChatRequest`

Serialize-only. Constructed solely through `ChatRequest::builder()`. Unset
optional fields are omitted from the wire.

| Builder method | Wire key | Type | Required |
|---|---|---|---|
| `model` | `model` | `ModelId` | ✅ |
| `messages` | `messages` | `Vec<Message>` | ✅ |
| `max_tokens` | `max_tokens` | `MaxTokens` | |
| `temperature` | `temperature` | `Temperature` | |
| `top_p` | `top_p` | `TopP` | |
| `top_k` | `top_k` | `TopK` | |
| `seed` | `seed` | `Seed` | |
| `frequency_penalty` | `frequency_penalty` | `FrequencyPenalty` | |
| `presence_penalty` | `presence_penalty` | `PresencePenalty` | |
| `repetition_penalty` | `repetition_penalty` | `RepetitionPenalty` | |
| `stop` | `stop` | `StopSequences` | |
| `user` | `user` | `impl Into<String>` | |
| `tools` | `tools` | `Vec<Tool>` | |
| `tool_choice` | `tool_choice` | `ToolChoice` | |
| `response_format` | `response_format` | `ResponseFormat` | |
| `provider` | `provider` | `ProviderPreferences` | |
| `models` | `models` | `Vec<ModelId>` | |
| `cache_control` | `cache_control` | `CacheControl` | |

There is a nineteenth field, `stream: bool`, which is `#[doc(hidden)]`,
crate-private, skipped when `false`, and set by the resource layer. It has no
builder method.

### Accessors

`model() -> &ModelId`, `messages() -> &[Message]`. `ChatRequest` derives
`Clone`, `Debug`, `PartialEq`, `Serialize`.

### Minimal serialisation

```rust
ChatRequest::builder()
    .model(ModelId::custom("openai/gpt-4o")?)
    .messages(vec![Message::user("hi")])
    .build()
```

```json
{"model": "openai/gpt-4o", "messages": [{"role": "user", "content": "hi"}]}
```

Exactly two keys — every other field is absent.

## `ChatRequestBuilder<M, Ms>`

```rust
pub struct ChatRequestBuilder<M, Ms> where M: FieldState, Ms: FieldState
```

`M` tracks `model`, `Ms` tracks `messages`; each is `Missing` or `Present`.
`FieldState` is sealed. `build()` is implemented only on
`ChatRequestBuilder<Present, Present>` and is infallible and `#[must_use]`.

The two required setters may be called in either order. Optional setters are
available in every state and survive the required-field transitions.

> `chat::Missing` / `chat::Present` and `builder::Missing` / `builder::Present`
> are distinct types with the same names, belonging to the request builder and
> the client builder respectively.

## `Message`

Fields are crate-private; build with the constructors.

| Constructor | Role | Content |
|---|---|---|
| `Message::new(role, content)` | explicit | `Some` |
| `Message::system(content)` | `system` | `Some` |
| `Message::user(content)` | `user` | `Some` |
| `Message::assistant(content)` | `assistant` | `Some` |
| `Message::tool(content)` | `tool` | `Some` |
| `Message::assistant_tool_calls(Vec<ToolCall>)` | `assistant` | **absent from the wire** |
| `Message::tool_result(ToolCallId, content)` | `tool` | `Some`, plus `tool_call_id` |

`with_name(impl Into<String>)` attaches an optional participant `name`.
Accessors: `role() -> Role`, `content() -> Option<&MessageContent>`.

Wire fields: `role`, and `content` / `name` / `tool_calls` / `tool_call_id` each
skipped when `None`.

### `Role`

`System` | `User` | `Assistant` | `Tool`. Serialises lowercase. Round-trips.

### `MessageContent`

An untagged enum:

| Variant | Wire |
|---|---|
| `Text(String)` | a JSON string |
| `Parts(Vec<ContentPart>)` | a JSON array |

`From` impls exist for `String`, `&str`, and `Vec<ContentPart>`, which is why
every constructor takes `impl Into<MessageContent>`.

### `ContentPart`

One variant in this release:

```rust
ContentPart::Text { text: String, cache_control: Option<CacheControl> }
```

Serialises as `{"type":"text","text":"…"}` plus `cache_control` when set.
Constructors: `ContentPart::text(t)` and `ContentPart::text_cached(t, cc)`.
There is no image, audio, or file part.

## Sampling parameter bounds

Enforced at newtype construction, not at request build. Full table in
[domain-types.md](domain-types.md#bounded-numerics).

| Type | Range |
|---|---|
| `MaxTokens` | any non-zero `u32` |
| `Temperature` | finite, `0.0..=2.0` |
| `TopP` | finite, `0.0..=1.0` |
| `TopK` | any `u32` (infallible) |
| `Seed` | any `u64` (infallible) |
| `FrequencyPenalty` | finite, `-2.0..=2.0` |
| `PresencePenalty` | finite, `-2.0..=2.0` |
| `RepetitionPenalty` | finite, `0.0..=2.0` |
| `StopSequences` | 1 to 4 entries |

Floating-point parameters are `f32`. `Price`, `ThroughputFloor`, and
`LatencyCeiling` are `f64`.

## `ChatResource<'a, T>`

Obtained from `Client::chat()`. Borrows the client; do not construct directly.

| Method | Returns |
|---|---|
| `send(ChatRequest)` | `Result<ChatCompletion, Error>` |
| `send_cached(ChatRequest, ResponseCache)` | `Result<Cached<ChatCompletion>, Error>` |
| `stream(ChatRequest)` | `Result<ChatStream, Error>` |
| `stream_cached(ChatRequest, ResponseCache)` | `Result<Cached<ChatStream>, Error>` |

All four share one request builder internally: same URL join, same auth and
attribution headers, differing only in the `Accept` header and whether the three
cache request headers are rendered. `stream` and `stream_cached` set
`stream: true` on a mutable copy of the request before serialising.

Error sets are identical across the four; see [errors.md](errors.md).

## Not modelled

Present in the OpenRouter chat API, absent from this crate:

- `n` (multiple choices per request). `ChatCompletion::choices` is a `Vec` and
  decodes whatever arrives, but nothing requests more than the default.
- `logprobs` / `top_logprobs` on the request side. The response field is decoded
  as an untyped `serde_json::Value`.
- `prediction`, `transforms`, `route`, `plugins`, `reasoning`.
- Non-text content parts.
- The object form of `provider.sort` (`{by, partition}`).
