# How to request structured outputs

Set `response_format` on the request to constrain the model's output shape.
There are two modes.

## JSON object mode

The loosest constraint: valid JSON, no schema.

```rust
use openrouter_rs::prelude::*;

let req = ChatRequest::builder()
    .model(ModelId::custom("openai/gpt-4o-mini")?)
    .messages(vec![Message::user("List three colours as a JSON array under key `colours`.")])
    .response_format(ResponseFormat::JsonObject)
    .build();
```

Serialises to `{"response_format": {"type": "json_object"}}`.

## JSON Schema mode

Supply a schema and, optionally, a strictness flag.

```rust
use openrouter_rs::prelude::*;
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": {
        "city": { "type": "string" },
        "temperature_c": { "type": "number" }
    },
    "required": ["city", "temperature_c"]
});

let mut cfg = JsonSchemaConfig::new(SchemaName::new("weather")?, schema);
cfg.strict = Some(SchemaStrictness::Strict);

let req = ChatRequest::builder()
    .model(ModelId::custom("openai/gpt-4o-mini")?)
    .messages(vec![Message::user("What is the weather in Paris?")])
    .response_format(ResponseFormat::JsonSchema(cfg))
    .build();
```

Serialises to:

```json
{
  "response_format": {
    "type": "json_schema",
    "json_schema": {
      "name": "weather",
      "strict": true,
      "schema": { "type": "object", "…": "…" }
    }
  }
}
```

`JsonSchemaConfig::new` leaves `strict` as `None`, which omits the key and lets
the provider apply its default. `strict` is a public field — set it directly, as
above. `SchemaStrictness` is a two-variant enum rather than a `bool` so a call
site reads as intent, not as a bare `true`:

| Variant | Wire value |
|---|---|
| `SchemaStrictness::Strict` | `true` |
| `SchemaStrictness::Lenient` | `false` |

`SchemaName` enforces `[A-Za-z0-9_-]`, 1–64 characters, at construction — the
same rule as `FunctionName`.

## Parse the result yourself

The crate does not decode structured output for you. The model's JSON arrives as
a plain string in `message.content`:

```rust
#[derive(serde::Deserialize)]
struct Weather { city: String, temperature_c: f64 }

let completion = client.chat().send(req).await?;
let raw = completion.choices.first()
    .and_then(|c| c.message.content.as_deref())
    .ok_or("no content returned")?;
let weather: Weather = serde_json::from_str(raw)?;
```

Nothing in the SDK verifies the response against the schema you sent. Enforcement
is the provider's job; a decode failure on your side is the signal that it did
not hold up its end.

## Choosing between the two

Use `JsonObject` when you only need parseable JSON and will validate the shape
yourself anyway. Use `JsonSchema` when you want the provider to constrain
generation — it costs you a schema to write, and support varies by model.
Combine `JsonSchema` with
[`ParameterRequirement::Required`](steer-provider-routing.md#require-parameter-support)
to route only to providers that honour the request rather than silently ignoring
it.
