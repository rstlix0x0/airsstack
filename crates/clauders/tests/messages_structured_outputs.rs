//! Structured Outputs round-trip: send `output_config`, receive a constrained
//! response or a refusal `stop_reason`.

#![cfg(all(feature = "messages-structured-outputs", feature = "transport-reqwest"))]
#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
)]
#![expect(
    clippy::panic,
    reason = "test-only panics on wrong-variant matches; a panic is the intended failure signal"
)]

use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clauders::messages::structured_outputs::OutputConfig;
use clauders::messages::{MessageRequest, StopReason};
use clauders::types::{ApiKey, BaseUrl, MaxTokens, ModelId};

const STRUCTURED_RESPONSE: &str = r#"{
    "id": "msg_01",
    "type": "message",
    "role": "assistant",
    "model": "claude-sonnet-4-5",
    "content": [{"type": "text", "text": "{\"name\":\"Alice\",\"age\":30}"}],
    "stop_reason": "end_turn",
    "stop_sequence": null,
    "usage": {"input_tokens": 30, "output_tokens": 12}
}"#;

const REFUSAL_RESPONSE: &str = r#"{
    "id": "msg_02",
    "type": "message",
    "role": "assistant",
    "model": "claude-sonnet-4-5",
    "content": [],
    "stop_reason": "refusal",
    "stop_sequence": null,
    "usage": {"input_tokens": 20, "output_tokens": 0}
}"#;

#[tokio::test]
async fn structured_output_round_trip() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(serde_json::json!({
            "output_config": {
                "format": { "type": "json_schema" }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_string(STRUCTURED_RESPONSE))
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age":  { "type": "integer" }
        },
        "required": ["name", "age"]
    });

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(64).unwrap())
        .output_config(OutputConfig::json_schema(schema))
        .add_user_text("Return a JSON object with name and age for Alice who is 30.")
        .build();

    let msg = client.messages().create(req).await.unwrap();
    assert_eq!(msg.stop_reason, Some(StopReason::EndTurn));

    // Verify the text content parses as the expected JSON.
    if let clauders::messages::ContentBlock::Text(tb) = &msg.content[0] {
        let parsed: serde_json::Value = serde_json::from_str(&tb.text).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 30);
    } else {
        panic!("expected a text content block");
    }
}

#[tokio::test]
async fn refusal_surfaces_as_stop_reason() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(REFUSAL_RESPONSE))
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(64).unwrap())
        .output_config(OutputConfig::json_schema(serde_json::json!({})))
        .add_user_text("Produce something the model refuses.")
        .build();

    // A refusal is a valid 200 response, not an error.
    let msg = client.messages().create(req).await.unwrap();
    assert_eq!(
        msg.stop_reason,
        Some(StopReason::Refusal),
        "stop_reason 'refusal' must decode as StopReason::Refusal"
    );
}
