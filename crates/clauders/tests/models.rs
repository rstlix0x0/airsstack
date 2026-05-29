//! Models resource integration tests against a local [`wiremock::MockServer`].
//!
//! These tests lock the `GET /v1/models` (list) and `GET /v1/models/{id}`
//! (get-by-id) request dispatch and response-decoding behaviour.

#![cfg(all(feature = "models", feature = "transport-reqwest"))]
#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
)]

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clauders::models::ModelInfoKind;
use clauders::types::{ApiKey, BaseUrl, ModelId};

const LIST_RESPONSE: &str = r#"{
    "data": [
        {
            "id": "claude-opus-4-7",
            "display_name": "Claude Opus 4.7",
            "created_at": "2026-01-01T00:00:00Z",
            "type": "model"
        },
        {
            "id": "claude-sonnet-4-5",
            "display_name": "Claude Sonnet 4.5",
            "created_at": "2025-09-01T00:00:00Z",
            "type": "model"
        }
    ],
    "has_more": false,
    "first_id": "claude-opus-4-7",
    "last_id": "claude-sonnet-4-5"
}"#;

const GET_RESPONSE: &str = r#"{
    "id": "claude-sonnet-4-5",
    "display_name": "Claude Sonnet 4.5",
    "created_at": "2025-09-01T00:00:00Z",
    "type": "model"
}"#;

#[tokio::test]
async fn list_returns_two_models() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LIST_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let list = client.models().list().await.unwrap();

    assert_eq!(list.data.len(), 2);
    assert!(!list.has_more);
    assert_eq!(list.data[0].kind, ModelInfoKind::Model);
    assert_eq!(list.data[0].display_name, "Claude Opus 4.7");
    assert_eq!(
        list.first_id,
        Some(ModelId::custom("claude-opus-4-7").unwrap())
    );
}

#[tokio::test]
async fn get_returns_single_model() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/models/claude-sonnet-4-5"))
        .respond_with(ResponseTemplate::new(200).set_body_string(GET_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let info = client
        .models()
        .get(&ModelId::claude_sonnet_4_5())
        .await
        .unwrap();

    assert_eq!(info.display_name, "Claude Sonnet 4.5");
    assert_eq!(info.kind, ModelInfoKind::Model);
    assert_eq!(info.id, ModelId::claude_sonnet_4_5());
}
