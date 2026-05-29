//! Tool round-trip: send tools, receive `tool_use`, send `tool_result`.

#![cfg(all(feature = "messages-tools", feature = "transport-reqwest"))]
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

use clauders::messages::tools::{Tool, ToolChoice, ToolResultBlock, ToolUseBlock};
use clauders::messages::{ContentBlock, MessageContent, MessageRequest, Role, StopReason};
use clauders::types::{ApiKey, BaseUrl, MaxTokens, ModelId, ToolName};

const TOOL_USE_RESPONSE: &str = r#"{
    "id":"msg_01",
    "type":"message",
    "role":"assistant",
    "model":"claude-sonnet-4-5",
    "content":[
        {"type":"tool_use","id":"toolu_01","name":"get_weather","input":{"city":"Paris"}}
    ],
    "stop_reason":"tool_use",
    "stop_sequence":null,
    "usage":{"input_tokens":40,"output_tokens":10}
}"#;

const FOLLOWUP_RESPONSE: &str = r#"{
    "id":"msg_02",
    "type":"message",
    "role":"assistant",
    "model":"claude-sonnet-4-5",
    "content":[{"type":"text","text":"It is sunny."}],
    "stop_reason":"end_turn",
    "stop_sequence":null,
    "usage":{"input_tokens":60,"output_tokens":4}
}"#;

#[tokio::test]
async fn tool_round_trip() {
    let server = MockServer::start().await;

    // First call returns tool_use.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(
            serde_json::json!({"tool_choice":{"type":"auto"}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(TOOL_USE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let tool = Tool {
        name: ToolName::new("get_weather").unwrap(),
        description: "Look up weather".into(),
        input_schema: serde_json::json!({
            "type":"object","properties":{"city":{"type":"string"}},"required":["city"]
        }),
    };

    let req1 = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(256).unwrap())
        .tools([tool])
        .tool_choice(ToolChoice::Auto)
        .add_user_text("What is the weather in Paris?")
        .build();

    let msg1 = client.messages().create(req1).await.unwrap();
    assert_eq!(msg1.stop_reason, Some(StopReason::ToolUse));

    let tool_use_id = match &msg1.content[0] {
        ContentBlock::ToolUse(ToolUseBlock { id, .. }) => id.clone(),
        other => panic!("expected ToolUse content, got {other:?}"),
    };

    // Second call returns the follow-up text.
    server.reset().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLLOWUP_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let req2 = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(256).unwrap())
        .add_user_text("What is the weather in Paris?")
        .add_assistant_text("(tool call) get_weather")
        .add_message(
            Role::User,
            MessageContent::Blocks(vec![ContentBlock::ToolResult(ToolResultBlock::text(
                tool_use_id.clone(),
                r#"{"temperature":24,"conditions":"sunny"}"#,
            ))]),
        )
        .build();

    let msg2 = client.messages().create(req2).await.unwrap();
    assert_eq!(msg2.stop_reason, Some(StopReason::EndTurn));
    if let ContentBlock::Text(tb) = &msg2.content[0] {
        assert!(tb.text.contains("sunny"));
    } else {
        panic!("expected Text response");
    }
}
