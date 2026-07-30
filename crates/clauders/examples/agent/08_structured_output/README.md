# 08 — Structured output

Make the agent's final answer a JSON document instead of prose, and cap what the run
may spend.

## Run

```text
cargo run -p clauders --example agent_08_structured_output
```

## What it shows

```rust
let options = Options::builder()
    .output_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "language":       { "type": "string" },
            "paradigms":      { "type": "array", "items": { "type": "string" } },
            "first_released": { "type": "integer" },
            "memory_safe":    { "type": "boolean" }
        },
        "required": ["language", "paradigms", "first_released", "memory_safe"],
        "additionalProperties": false
    }))
    .max_budget_usd(BudgetUsd::new(0.50)?)
    .build();
```

`output_schema(value)` is the convenience form. `output_format(OutputConfig)` takes
the full config — the same `OutputConfig` the Messages API uses — when a schema name
or strictness flag is also needed.

## Reading the result

```rust
match &result.structured_output {
    Some(value) => println!("{}", serde_json::to_string_pretty(value)?),
    None => println!("(no structured output; raw result: {})", result.result),
}
```

`ResultMessage::structured_output` is `Option<serde_json::Value>` — already parsed.
`None` means either no schema was requested, or the run ended before one could be
produced. It is ordinary `serde_json`, so it deserializes into a Rust type the usual
way.

## Budget

```rust
.max_budget_usd(BudgetUsd::new(0.50)?)
```

A **client-side** spend ceiling. `BudgetUsd::new` rejects NaN, infinity, and anything
`<= 0.0` at construction, so an invalid budget is an error where you wrote it rather
than a surprise at connect. There is no upper bound — the binary enforces the ceiling
and ends the run when its own cost estimate reaches it.

## Why the turn stopped

`ResultSubtype` is worth handling explicitly once, because two of its arms only exist
because of options set in this example:

```rust
match &result.subtype {
    ResultSubtype::Success => {}
    ResultSubtype::ErrorMaxBudgetUsd => { /* hit max_budget_usd */ }
    ResultSubtype::ErrorMaxTurns => { /* hit max_turns */ }
    ResultSubtype::ErrorMaxStructuredOutputRetries => { /* could not match the schema */ }
    ResultSubtype::ErrorDuringExecution => { /* something failed mid-run */ }
    other => println!("subtype not modelled by this release: {other:?}"),
}
```

The enum is `#[non_exhaustive]` **and** carries an `Unknown(String)` arm, so a newer
binary's subtype keeps its wire name and stays matchable instead of failing the
decode. That is why the wildcard arm is required and why it can still print something
useful.

`result.errors` carries diagnostics on an error result; it defaults to empty on a
success frame rather than failing to decode.
