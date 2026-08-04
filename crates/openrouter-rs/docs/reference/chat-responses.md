# Reference — Chat responses

Deserialize-only data carriers with public fields. Unknown fields are ignored on
decode, so a server-side addition never breaks an existing client.

## `ChatCompletion`

| Field | Type | Notes |
|---|---|---|
| `id` | `String` | generation id (`gen-…`) |
| `object` | `String` | `"chat.completion"` |
| `created` | `u64` | Unix seconds |
| `model` | `String` | as echoed by the server, **not** a `ModelId` |
| `choices` | `Vec<Choice>` | |
| `usage` | `Option<Usage>` | absent unless reported |
| `system_fingerprint` | `Option<String>` | absent unless reported |

Derives `Clone`, `Debug`, `PartialEq`, `Deserialize`.

## `Choice`

| Field | Type | Notes |
|---|---|---|
| `index` | `u32` | |
| `message` | `ResponseMessage` | |
| `finish_reason` | `Option<FinishReason>` | |
| `logprobs` | `Option<serde_json::Value>` | passed through untyped |

## `ResponseMessage`

| Field | Type | Notes |
|---|---|---|
| `role` | `Role` | `assistant` for model output |
| `content` | `Option<String>` | `None` when the server sends `null` — normal on a tool-calling turn |
| `tool_calls` | `Option<Vec<ToolCall>>` | present when the model chose to use tools |

Note the asymmetry with the request side: request `Message` carries a
`MessageContent` (string *or* parts); response `ResponseMessage` carries a plain
`Option<String>`.

## `FinishReason`

| Variant | Wire |
|---|---|
| `Stop` | `"stop"` |
| `Length` | `"length"` |
| `ToolCalls` | `"tool_calls"` |
| `ContentFilter` | `"content_filter"` |
| `Error` | `"error"` |
| `Unknown` | anything else |

`Unknown` is `#[serde(other)]`: an unrecognised server value decodes to it
rather than failing. Deserialize-only — there is no `Serialize` impl.

## `Usage`

| Field | Type | Notes |
|---|---|---|
| `prompt_tokens` | `u32` | required |
| `completion_tokens` | `u32` | required |
| `total_tokens` | `u32` | required |
| `cost` | `Option<f64>` | credit cost, when reported |
| `cache_discount` | `Option<f64>` | negative on a prompt-cache write, positive on reads |
| `cost_details` | `Option<CostDetails>` | |
| `is_byok` | `Option<bool>` | bring-your-own-key provider |
| `prompt_tokens_details` | `Option<PromptTokensDetails>` | |
| `completion_tokens_details` | `Option<CompletionTokensDetails>` | |

Every optional field defaults to `None`, so a bare three-field usage block
decodes cleanly.

### `PromptTokensDetails`

`cached_tokens`, `cache_write_tokens`, `audio_tokens`, `video_tokens` — all
`Option<u32>`. The first two are the prompt-cache statistics: reads and writes
respectively.

### `CompletionTokensDetails`

`reasoning_tokens`, `audio_tokens`, `accepted_prediction_tokens`,
`rejected_prediction_tokens` — all `Option<u32>`.

### `CostDetails`

`upstream_inference_cost`, `upstream_inference_prompt_cost`,
`upstream_inference_completions_cost` — all `Option<f64>`, USD.

All three detail structs derive `Default` and decode an empty object to
all-`None`.

## `ToolCall` and `FunctionCall`

Shared between request (assistant replay) and response decode — one definition,
so the wire shapes cannot drift apart. Both derive `Serialize` **and**
`Deserialize`.

```rust
pub struct ToolCall {
    pub id: ToolCallId,
    pub r#type: ToolType,       // always Function
    pub function: FunctionCall,
}

pub struct FunctionCall {
    pub name: String,           // plain String, not FunctionName
    pub arguments: String,      // raw JSON string, NOT a parsed object
}
```

`arguments` is the unparsed JSON text exactly as the server sent it —
`"{\"q\":\"rust\"}"`, not a `serde_json::Value`. That preserves the payload
byte-for-byte and avoids a lossy round trip. Deserialise it yourself.

`ToolType` has a hand-written `Deserialize` that accepts only `"function"` and
rejects anything else as an unknown variant.

## Where each type is exported from

| Type | Crate root | `chat` module | prelude |
|---|---|---|---|
| `ChatCompletion` | ✅ | ✅ | ✅ |
| `FinishReason` | ✅ | ✅ | ✅ |
| `ToolCall`, `FunctionCall` | ✅ | ✅ | ✅ |
| `Usage` | ❌ | ✅ | ✅ |
| `Choice`, `ResponseMessage` | ❌ | ✅ | ❌ |
| `PromptTokensDetails` and siblings | ✅ | ✅ | ❌ |

Anything not in the prelude is reachable as `openrouter_rs::chat::<Name>`.
