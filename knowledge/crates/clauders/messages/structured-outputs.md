---
type: Rust Module
title: clauders::messages::structured_outputs
description: Structured Outputs support — OutputConfig/OutputFormat constrain a Messages API response to a caller-supplied JSON Schema.
tags: [rust, sdk, anthropic, messages-api, structured-outputs, json-schema]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/messages/structured_outputs.rs
---

Gated behind `messages-structured-outputs` (depends on `messages`, not
enabled by default). Distinct from `Tool.strict` (in
[tools.rs](/crates/clauders/messages/tools.md)), which constrains tool
*input* rather than the top-level response.

# Schema

```rust
pub struct OutputConfig { pub format: OutputFormat }

#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputFormat {
    JsonSchema { schema: serde_json::Value },
}
```

`OutputConfig::json_schema(schema: serde_json::Value) -> Self` is the
convenience constructor for the common case. The SDK does not pre-validate
the schema; the API enforces conformance at inference time.
[StopReason::Refusal](/crates/clauders/messages/response.md) signals the
model declined to produce the constrained output.

# Examples

```rust
use clauders::messages::structured_outputs::OutputConfig;
let cfg = OutputConfig::json_schema(serde_json::json!({
    "type": "object",
    "properties": { "name": { "type": "string" } },
    "required": ["name"]
}));
let j = serde_json::to_value(&cfg).unwrap();
assert_eq!(j["format"]["type"], "json_schema");
```

Related: [MessageRequestBuilder::output_config](/crates/clauders/messages/request.md),
[Tool::strict](/crates/clauders/messages/tools.md),
[StopReason](/crates/clauders/messages/response.md).

# Citations

1. `crates/clauders/src/messages/structured_outputs.rs`
