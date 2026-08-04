# Reference — Tools and structured outputs

Two independent request features that happen to share a shape: both send a JSON
Schema and both are serialise-only.

---

## Tools

Module `chat::tool` (request side) and `chat::tool_call` (shared carrier).

### `Tool`

```rust
pub struct Tool {
    pub r#type: ToolType,
    pub function: FunctionDef,
}
```

`Tool::function(def)` is a `const` constructor that fills in
`ToolType::Function`. Serialise-only.

```json
{"type": "function", "function": {"name": "search"}}
```

### `ToolType`

One variant, `Function` → `"function"`. `Serialize` is derived; `Deserialize` is
hand-written and rejects anything other than `"function"` as an unknown variant.

### `FunctionDef`

Public fields; the three optionals are skipped when `None`.

| Field | Type | Notes |
|---|---|---|
| `name` | `FunctionName` | validated: `[A-Za-z0-9_-]`, 1–64 chars |
| `description` | `Option<String>` | what the model uses to decide when to call |
| `parameters` | `Option<serde_json::Value>` | a JSON Schema object; the crate does not generate it |
| `strict` | `Option<bool>` | a plain `bool` here, unlike `SchemaStrictness` on the structured-output side |

`FunctionDef::new(name)` is `const` and leaves all three optionals `None`.

### `ToolChoice`

Hand-written `Serialize`; the variant decides between a JSON string and an
object.

| Variant | Wire |
|---|---|
| `None` | `"none"` |
| `Auto` | `"auto"` |
| `Required` | `"required"` |
| `Function { name: FunctionName }` | `{"type":"function","function":{"name":"…"}}` |

Omitting `tool_choice` from the request entirely also lets the model decide.

### `ToolCall` and `FunctionCall`

The shared carrier — see
[chat-responses.md](chat-responses.md#toolcall-and-functioncall) for the full
description. The key point: `FunctionCall::arguments` is a **raw JSON string**,
not a parsed object.

### Round-trip message shapes

| Step | Constructor | Wire |
|---|---|---|
| 1. Ask | `Message::user(q)` | `{"role":"user","content":"…"}` |
| 2. Model calls | (decoded) | `message.tool_calls[]`, `content: null`, `finish_reason: "tool_calls"` |
| 3. Replay | `Message::assistant_tool_calls(calls)` | `{"role":"assistant","tool_calls":[…]}` — **no `content` key** |
| 4. Answer | `Message::tool_result(id, out)` | `{"role":"tool","tool_call_id":"…","content":"…"}` |

Step 3's absent `content` key is load-bearing: the field is
`skip_serializing_if = "Option::is_none"` and the constructor sets it to `None`,
so an empty string never goes on the wire.

### Streaming limitation

`ChunkDelta` carries only `role` and `content`. There is no streamed tool-call
delta.

---

## Structured outputs

Module `chat::response_format`. Set with
`ChatRequestBuilder::response_format(fmt)`.

### `ResponseFormat`

Hand-written `Serialize`.

| Variant | Wire |
|---|---|
| `JsonObject` | `{"type":"json_object"}` |
| `JsonSchema(JsonSchemaConfig)` | `{"type":"json_schema","json_schema":{…}}` |

### `JsonSchemaConfig`

```rust
pub struct JsonSchemaConfig {
    pub name: SchemaName,
    pub strict: Option<SchemaStrictness>,
    pub schema: serde_json::Value,
}
```

`JsonSchemaConfig::new(name, schema)` is `const` and leaves `strict` as `None`.
Hand-written `Serialize` emits `name`, then `strict` only when set, then
`schema`.

### `SchemaStrictness`

| Variant | Wire |
|---|---|
| `Strict` | `true` |
| `Lenient` | `false` |

`as_bool()` converts. A two-variant enum rather than a `bool` so the call site
reads as intent. `None` omits the key and defers to the provider.

### Decoding is yours

Nothing in the SDK parses structured output or validates it against the schema
you sent. The model's JSON arrives as a plain `String` in
`ResponseMessage::content`; deserialise it yourself.

### `SchemaName`

Same validation as `FunctionName`: `[A-Za-z0-9_-]`, 1–64 characters, non-empty.
Distinct types so the compiler will not let you swap them.
