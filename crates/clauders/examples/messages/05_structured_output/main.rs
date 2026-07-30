//! Structured output — constrain the response to a JSON Schema.
//!
//! Sends `output_config` with a JSON Schema. The model's reply is a single
//! text block whose content is JSON conforming to that schema, so it parses
//! without any extra validation on the caller's side.
//!
//! Structured output is requested with the `anthropic-beta` header
//! `structured-outputs-2025-11-13`, attached on the client builder below.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 05_structured_output
//! ```

use clauders::messages::ContentBlock;
use clauders::messages::structured_outputs::OutputConfig;
use clauders::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let beta = BetaHeader::new("structured-outputs-2025-11-13")?;
    let client = Client::builder()?
        .api_key(api_key)
        .add_anthropic_beta(beta)
        .build()?;
    let max_tokens = MaxTokens::new(1024);

    // The schema the reply must satisfy. The API enforces conformance at
    // inference time; the SDK forwards the schema verbatim.
    // Strict schema: every object must set additionalProperties: false, or the
    // API rejects the request with an invalid_request_error.
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
        .max_tokens(max_tokens)
        .add_user_text("Give the capital of France as structured data.")
        .output_config(OutputConfig::json_schema(schema))
        .build();

    let msg = client.messages().create(req).await?;

    // The structured result arrives as JSON text in the response text block.
    for block in &msg.content {
        if let ContentBlock::Text(t) = block {
            let parsed: serde_json::Value = serde_json::from_str(&t.text)?;
            println!("{}", serde_json::to_string_pretty(&parsed)?);
        }
    }

    println!("stop_reason: {:?}", msg.stop_reason);
    Ok(())
}
