//! Make the agent's *final answer* a JSON document, and cap what the run may
//! cost.
//!
//! `output_schema` constrains the terminal result to a JSON Schema. The parsed
//! value arrives on `ResultMessage::structured_output`, so a caller gets data
//! instead of prose to parse.
//!
//! `max_budget_usd` is a client-side spend ceiling. When the run reaches it the
//! turn ends with `ResultSubtype::ErrorMaxBudgetUsd` — one of several error
//! subtypes worth handling explicitly, all shown below.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_08_structured_output
//! ```

use clauders::agent::message::ResultSubtype;
use clauders::agent::types::BudgetUsd;
use clauders::agent::{Message, Options, PermissionMode, query};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "language":   { "type": "string" },
            "paradigms":  { "type": "array", "items": { "type": "string" } },
            "first_released": { "type": "integer" },
            "memory_safe": { "type": "boolean" }
        },
        "required": ["language", "paradigms", "first_released", "memory_safe"],
        "additionalProperties": false
    });

    let options = Options::builder()
        .system_prompt("Answer only through the required output schema.")
        .permission_mode(PermissionMode::Plan)
        // Convenience for the common case. `output_format(OutputConfig)` takes
        // the full config when a name or strictness flag is also needed.
        .output_schema(schema)
        .max_budget_usd(BudgetUsd::new(0.50)?)
        .max_turns(3)
        .build();

    let mut stream = query("Describe the Rust programming language.", options).await?;

    while let Some(frame) = stream.next().await {
        if let Message::Result(result) = frame? {
            // Handle the closed set of terminal subtypes. The enum is
            // `#[non_exhaustive]` plus carries an `Unknown` arm, so a newer
            // binary's subtype is inspectable rather than a decode failure.
            match &result.subtype {
                ResultSubtype::Success => {}
                ResultSubtype::ErrorMaxBudgetUsd => {
                    println!("stopped: hit the $0.50 ceiling");
                }
                ResultSubtype::ErrorMaxTurns => println!("stopped: hit the turn cap"),
                ResultSubtype::ErrorMaxStructuredOutputRetries => {
                    println!("stopped: could not produce output matching the schema");
                }
                ResultSubtype::ErrorDuringExecution => {
                    println!("stopped: execution error");
                }
                other => println!("stopped: subtype not modelled by this release: {other:?}"),
            }

            for error in &result.errors {
                println!("error: {error}");
            }

            // The parsed document. `None` means no schema was requested, or the
            // run ended before one could be produced.
            match &result.structured_output {
                Some(value) => {
                    println!("--- structured output ---");
                    println!("{}", serde_json::to_string_pretty(value)?);

                    // It is ordinary `serde_json`, so it deserializes into a
                    // Rust type the usual way.
                    if let Some(paradigms) =
                        value.get("paradigms").and_then(serde_json::Value::as_array)
                    {
                        println!("paradigm count: {}", paradigms.len());
                    }
                }
                None => println!("(no structured output; raw result: {})", result.result),
            }

            if let Some(cost) = result.total_cost_usd {
                println!("cost: ${cost:.6} of the $0.50 ceiling");
            }
        }
    }

    Ok(())
}
