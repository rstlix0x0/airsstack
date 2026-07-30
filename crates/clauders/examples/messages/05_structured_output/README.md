# 05 — Structured output

Constrain the reply to a JSON Schema, so the response parses directly into data
with no extra validation.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 05_structured_output
```

## What it shows

**Attach the beta header** structured output requires, then **set the output
config** with a JSON Schema:

```rust
use clauders::messages::structured_outputs::OutputConfig;

let beta = BetaHeader::new("structured-outputs-2025-11-13")?;
let client = Client::builder()?
    .api_key(api_key)
    .add_anthropic_beta(beta)
    .build()?;

let schema = serde_json::json!({
    "type": "object",
    "properties": {
        "city":       { "type": "string" },
        "country":    { "type": "string" },
        "population": { "type": "integer" }
    },
    "required": ["city", "country", "population"],
    "additionalProperties": false
});

let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_5())
    .max_tokens(MaxTokens::new(1024))
    .add_user_text("Give the capital of France as structured data.")
    .output_config(OutputConfig::json_schema(schema))
    .build();
```

The reply arrives as a normal `ContentBlock::Text` whose content is JSON matching
the schema, so parse it straight back:

```rust
if let ContentBlock::Text(t) = block {
    let parsed: serde_json::Value = serde_json::from_str(&t.text)?;
    println!("{}", serde_json::to_string_pretty(&parsed)?);
}
```

## Notes

- **Strict schema is required.** Every `object` in the schema must set
  `"additionalProperties": false`, or the API returns an
  `invalid_request_error`.
- The crate serializes the GA `output_config.format` shape. The
  `structured-outputs-2025-11-13` beta header is what the live API currently
  expects; if a future API version rejects it, drop the `add_anthropic_beta`
  line — the request body is already GA-shaped.
- `OutputConfig` also carries a reasoning-`effort` slot; see `OutputConfig::effort`
  / `with_effort`.
