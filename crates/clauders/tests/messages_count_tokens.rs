//! Token-counting integration tests against a local [`wiremock::MockServer`].
//!
//! These tests lock the request-body filtering (the count-tokens endpoint
//! rejects fields the messages endpoint accepts, such as `max_tokens`) and
//! the response-decoding behaviour.

#![cfg(all(feature = "messages-token-counting", feature = "transport-reqwest"))]
#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
)]

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clauders::messages::MessageRequest;
use clauders::messages::token_counting::TokenCount;
use clauders::types::{ApiKey, BaseUrl, MaxTokens, ModelId};

#[tokio::test]
async fn count_tokens_returns_input_token_count() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages/count_tokens"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"input_tokens":42}"#))
        .expect(1)
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
        .max_tokens(MaxTokens::new(8).unwrap())
        .add_user_text("hello")
        .build();

    let tc: TokenCount = client.messages().count_tokens(req).await.unwrap();
    assert_eq!(tc.input_tokens, 42);
}

/// Verify that the wire body sent to the count-tokens endpoint does NOT
/// contain `max_tokens`. The Anthropic API rejects requests with fields it
/// does not recognise, and `max_tokens` is not accepted by this endpoint.
///
/// The assertion is made against the body the mock server actually received,
/// not a locally re-serialized copy, so a future change that sends the wrong
/// fields will make this test fail even if the projection type is private.
#[tokio::test]
async fn count_tokens_body_omits_max_tokens() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages/count_tokens"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"input_tokens":10}"#))
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
        .max_tokens(MaxTokens::new(512).unwrap())
        .add_user_text("count me")
        .build();

    client.messages().count_tokens(req).await.unwrap();

    // Inspect the body the server actually received to assert field presence
    // and absence. This is the real regression guard: if the wire body ever
    // includes `max_tokens`, the count-tokens endpoint will reject the request
    // in production.
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "expected exactly one request to the server"
    );

    let body_bytes = &requests[0].body;
    let body_json: serde_json::Value = serde_json::from_slice(body_bytes).unwrap();

    assert!(
        body_json.get("model").is_some(),
        "wire body must contain `model`"
    );
    assert!(
        body_json.get("messages").is_some(),
        "wire body must contain `messages`"
    );
    assert!(
        body_json.get("max_tokens").is_none(),
        "wire body must NOT contain `max_tokens` — count-tokens endpoint rejects it"
    );
    assert!(
        body_json.get("temperature").is_none(),
        "wire body must NOT contain `temperature`"
    );
    assert!(
        body_json.get("stream").is_none(),
        "wire body must NOT contain `stream`"
    );
}
