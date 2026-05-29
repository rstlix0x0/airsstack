//! End-to-end integration tests for `MessagesResource::create` over a local
//! [`wiremock::MockServer`].
//!
//! These tests lock the request-header propagation and response-decoding
//! behaviour without hitting the real Anthropic API.

#![cfg(all(feature = "messages", feature = "transport-reqwest"))]
#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
)]

use std::time::Duration;

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clauders::messages::{Role, StopReason};
use clauders::types::{ApiKey, BaseUrl, MaxTokens, ModelId};

/// Canned 200 response body from the Anthropic Messages API.
const HAPPY_BODY: &str = r#"{"id":"msg_01","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"Hello!"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":12,"output_tokens":6}}"#;

#[tokio::test]
async fn create_happy_path() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-test-abc"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("request-id", "req_01")
                .insert_header("anthropic-organization-id", "org_42")
                .set_body_string(HAPPY_BODY),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test-abc").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let req = clauders::messages::MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(64).unwrap())
        .add_user_text("Hi")
        .build();

    let msg = client.messages().create(req).await.unwrap();

    assert_eq!(msg.role, Role::Assistant);
    assert_eq!(msg.stop_reason, Some(StopReason::EndTurn));
    assert_eq!(msg.content.len(), 1);
    assert_eq!(msg.usage.input_tokens, 12);
}

#[tokio::test]
async fn create_decodes_429_as_retryable_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "5")
                .insert_header("request-id", "req_02")
                .set_body_string(
                    r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#,
                ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test-abc").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let req = clauders::messages::MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(64).unwrap())
        .add_user_text("Hi")
        .build();

    let err = client.messages().create(req).await.unwrap_err();

    assert!(err.is_retryable(), "429 rate_limit_error must be retryable");
    assert_eq!(
        err.retry_after(),
        Some(Duration::from_secs(5)),
        "retry_after must parse the Retry-After header"
    );
    assert_eq!(
        err.request_id().map(|r| r.as_str().to_owned()),
        Some("req_02".to_owned()),
        "request_id must propagate from the response header"
    );
}
