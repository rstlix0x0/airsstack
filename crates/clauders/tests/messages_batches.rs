//! End-to-end integration tests for the Message Batches API over a local
//! [`wiremock::MockServer`].
//!
//! Tests cover create, get, results streaming, and delete against canned
//! server responses, verifying request routing and response decoding.

#![cfg(all(feature = "messages-batches", feature = "transport-reqwest"))]
#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
)]

use std::pin::Pin;

use futures_core::Stream;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clauders::messages::{BatchRequest, BatchResult, BatchStatus};
use clauders::types::{ApiKey, BaseUrl, BatchId, CustomRequestId, MaxTokens, ModelId};

const CREATE_RESPONSE: &str = r#"{
    "id":"msgbatch_01",
    "type":"message_batch",
    "processing_status":"in_progress",
    "request_counts":{"processing":2,"succeeded":0,"errored":0,"canceled":0,"expired":0},
    "ended_at":null,
    "created_at":"2026-05-28T00:00:00Z",
    "expires_at":"2026-05-29T00:00:00Z",
    "archived_at":null,
    "cancel_initiated_at":null,
    "results_url":null
}"#;

const GET_ENDED_RESPONSE: &str = r#"{
    "id":"msgbatch_01",
    "type":"message_batch",
    "processing_status":"ended",
    "request_counts":{"processing":0,"succeeded":2,"errored":0,"canceled":0,"expired":0},
    "ended_at":"2026-05-28T00:10:00Z",
    "created_at":"2026-05-28T00:00:00Z",
    "expires_at":"2026-05-29T00:00:00Z",
    "archived_at":null,
    "cancel_initiated_at":null,
    "results_url":"https://example.com/v1/messages/batches/msgbatch_01/results"
}"#;

const RESULTS_JSONL: &str = concat!(
    r#"{"custom_id":"r1","result":{"type":"succeeded","message":{"id":"msg_01","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"a"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":1}}}}"#,
    "\n",
    r#"{"custom_id":"r2","result":{"type":"errored","error":{"type":"invalid_request_error","message":"bad input"}}}"#,
    "\n",
);

const DELETE_RESPONSE: &str = r#"{"id":"msgbatch_01","type":"message_batch_deleted"}"#;

#[tokio::test]
async fn batches_round_trip() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages/batches"))
        .respond_with(ResponseTemplate::new(200).set_body_string(CREATE_RESPONSE))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/messages/batches/msgbatch_01"))
        .respond_with(ResponseTemplate::new(200).set_body_string(GET_ENDED_RESPONSE))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/messages/batches/msgbatch_01/results"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/x-jsonl")
                .set_body_string(RESULTS_JSONL),
        )
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1/messages/batches/msgbatch_01"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DELETE_RESPONSE))
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let req_inner = clauders::messages::MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(8).unwrap())
        .add_user_text("hi")
        .build();

    let batch_req = BatchRequest::builder()
        .add(CustomRequestId::new("r1").unwrap(), req_inner.clone())
        .add(CustomRequestId::new("r2").unwrap(), req_inner)
        .build();

    // Create
    let created = client.messages().batches().create(batch_req).await.unwrap();
    assert_eq!(created.id.as_str(), "msgbatch_01");
    assert_eq!(created.processing_status, BatchStatus::InProgress);
    assert_eq!(created.request_counts.processing, 2);

    // Get
    let polled = client.messages().batches().get(&created.id).await.unwrap();
    assert_eq!(polled.processing_status, BatchStatus::Ended);
    assert_eq!(polled.request_counts.succeeded, 2);

    // Results
    let mut stream = client
        .messages()
        .batches()
        .results(&created.id)
        .await
        .unwrap();
    let mut ok = 0u32;
    let mut err = 0u32;
    while let Some(row) = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await {
        match row.unwrap().result {
            BatchResult::Succeeded { .. } => ok += 1,
            BatchResult::Errored { .. } => err += 1,
            _ => {}
        }
    }
    assert_eq!(ok, 1);
    assert_eq!(err, 1);

    // Delete
    let deleted = client
        .messages()
        .batches()
        .delete(&created.id)
        .await
        .unwrap();
    assert_eq!(deleted.id.as_str(), "msgbatch_01");
}

#[tokio::test]
async fn batches_list_returns_empty_page() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/messages/batches"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"data":[],"has_more":false,"first_id":null,"last_id":null}"#),
        )
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let list = client.messages().batches().list().await.unwrap();
    assert!(!list.has_more);
    assert!(list.data.is_empty());
}

#[tokio::test]
async fn batches_cancel_returns_batch() {
    let server = MockServer::start().await;

    let canceling_response = CREATE_RESPONSE.replace(r#""in_progress""#, r#""canceling""#);

    Mock::given(method("POST"))
        .and(path("/v1/messages/batches/msgbatch_01/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_string(canceling_response))
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let id = BatchId::new("msgbatch_01").unwrap();
    let batch = client.messages().batches().cancel(&id).await.unwrap();
    assert_eq!(batch.processing_status, BatchStatus::Canceling);
}
