//! Caching round-trip: send `cache_control` on the system prompt, receive
//! cache token counts in the response `Usage`.

#![cfg(all(feature = "messages-caching", feature = "transport-reqwest"))]
#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
)]

use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clauders::messages::MessageRequest;
use clauders::types::{
    ApiKey, BaseUrl, CacheControl, MaxTokens, ModelId, SystemPrompt, SystemSegment,
};

const CACHED_RESPONSE: &str = r#"{
    "id":"msg_01",
    "type":"message",
    "role":"assistant",
    "model":"claude-sonnet-4-5",
    "content":[{"type":"text","text":"OK"}],
    "stop_reason":"end_turn",
    "stop_sequence":null,
    "usage":{
        "input_tokens":20,
        "output_tokens":2,
        "cache_creation_input_tokens":500,
        "cache_read_input_tokens":100,
        "cache_creation":{"ephemeral_5m_input_tokens":300,"ephemeral_1h_input_tokens":200}
    }
}"#;

#[tokio::test]
async fn cache_control_round_trip() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(serde_json::json!({
            "system": [
                {
                    "type": "text",
                    "text": "You are terse.",
                    "cache_control": {"type": "ephemeral"}
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_string(CACHED_RESPONSE))
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let seg = SystemSegment::text("You are terse.").with_cache(CacheControl::ephemeral());

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(8).unwrap())
        .system(SystemPrompt::segments(vec![seg]))
        .add_user_text("hi")
        .build();

    let msg = client.messages().create(req).await.unwrap();
    assert_eq!(msg.usage.cache_creation_input_tokens, Some(500));
    assert_eq!(msg.usage.cache_read_input_tokens, Some(100));
    assert_eq!(msg.usage.total_input_tokens(), 20 + 500 + 100);

    let cc = msg.usage.cache_creation.unwrap();
    assert_eq!(cc.ephemeral_5m_input_tokens, 300);
    assert_eq!(cc.ephemeral_1h_input_tokens, 200);
}
