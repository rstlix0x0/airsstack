//! End-to-end SSE streaming tests against a local mock HTTP server.

#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
)]
#![expect(
    clippy::panic,
    reason = "test-only panics on unexpected outcomes; a panic is the intended failure signal"
)]

use futures_core::Stream;
use std::pin::Pin;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use clauders::messages::{ContentBlock, ContentDelta, StopReason, StreamEvent};
use clauders::types::{ApiKey, BaseUrl, MaxTokens, ModelId};

const SSE_FULL: &str = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_01\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-5\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":12,\"output_tokens\":1}}}\n\
\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" world\"}}\n\
\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\
\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":6}}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
\n";

#[tokio::test]
async fn stream_yields_full_event_sequence() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(SSE_FULL),
        )
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let req = clauders::messages::MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(64))
        .add_user_text("hi")
        .build();

    let mut stream = client.messages().stream(req).await.unwrap();
    let mut kinds: Vec<&'static str> = Vec::new();

    loop {
        let next = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        match next {
            None => break,
            Some(Ok(e)) => {
                kinds.push(match e {
                    StreamEvent::MessageStart { .. } => "message_start",
                    StreamEvent::ContentBlockStart { .. } => "content_block_start",
                    StreamEvent::ContentBlockDelta {
                        delta: ContentDelta::TextDelta { .. },
                        ..
                    } => "text_delta",
                    StreamEvent::ContentBlockDelta { .. } => "other_delta",
                    StreamEvent::ContentBlockStop { .. } => "content_block_stop",
                    StreamEvent::MessageDelta { .. } => "message_delta",
                    StreamEvent::MessageStop => "message_stop",
                    StreamEvent::Ping => "ping",
                    StreamEvent::Error { .. } => "error",
                    _ => "unknown",
                });
            }
            Some(Err(e)) => panic!("unexpected stream error: {e}"),
        }
    }

    assert_eq!(
        kinds,
        vec![
            "message_start",
            "content_block_start",
            "text_delta",
            "text_delta",
            "content_block_stop",
            "message_delta",
            "message_stop",
        ]
    );
}

#[tokio::test]
async fn stream_collect_assembles_message() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(SSE_FULL),
        )
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let req = clauders::messages::MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(64))
        .add_user_text("hi")
        .build();

    let msg = client
        .messages()
        .stream(req)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();

    assert_eq!(msg.usage.output_tokens, 6);
    match msg.content.first() {
        Some(ContentBlock::Text(tb)) => assert_eq!(tb.text, "Hello world"),
        other => panic!("expected text block, got {other:?}"),
    }
}

#[tokio::test]
async fn mid_stream_error_is_terminal() {
    let sse_body = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_01\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-5\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\
\n\
event: error\n\
data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"please retry\"}}\n\
\n";

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let req = clauders::messages::MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(8))
        .add_user_text("hi")
        .build();

    let mut stream = client.messages().stream(req).await.unwrap();

    // First event: message_start (success).
    let e1 = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx))
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(e1, StreamEvent::MessageStart { .. }));

    // Second event: error (the stream delivers it as StreamEvent::Error).
    let e2 = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx))
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(e2, StreamEvent::Error { .. }));

    // Stream is now terminal — next poll must return None.
    let none = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
    assert!(
        none.is_none(),
        "expected terminal None after error event, got {none:?}"
    );
}

const SSE_TOOL_USE: &str = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_02\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-5\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":9,\"output_tokens\":0}}}\n\
\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_01\",\"name\":\"get_weather\",\"input\":{}}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"city\\\":\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"Paris\\\"}\"}}\n\
\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\
\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":11}}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
\n";

const SSE_THINKING_AND_UNKNOWN: &str = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_03\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-5\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":4,\"output_tokens\":0}}}\n\
\n\
event: some_future_event\n\
data: {\"type\":\"some_future_event\",\"payload\":{\"a\":1}}\n\
\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"step one \"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"step two\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig-abc\"}}\n\
\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
\n";

async fn collect_sse(body: &'static str) -> Result<clauders::messages::Message, clauders::Error> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let client = clauders::Client::builder()
        .unwrap()
        .api_key(ApiKey::new("sk-test").unwrap())
        .base_url(BaseUrl::parse(server.uri()).unwrap())
        .build()
        .unwrap();

    let req = clauders::messages::MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(64))
        .add_user_text("hi")
        .build();

    client.messages().stream(req).await.unwrap().collect().await
}

#[tokio::test]
async fn stream_collect_assembles_tool_use_arguments() {
    let msg = collect_sse(SSE_TOOL_USE).await.unwrap();
    match msg.content.first() {
        Some(ContentBlock::ToolUse(tu)) => {
            assert_eq!(tu.name.as_str(), "get_weather");
            assert_eq!(tu.input, serde_json::json!({"city": "Paris"}));
        }
        other => panic!("expected tool-use block, got {other:?}"),
    }
    assert_eq!(msg.stop_reason, Some(StopReason::ToolUse));
    assert_eq!(msg.usage.output_tokens, 11);
}

#[tokio::test]
async fn stream_collect_assembles_thinking_and_survives_unknown_events() {
    let msg = collect_sse(SSE_THINKING_AND_UNKNOWN).await.unwrap();
    match msg.content.first() {
        Some(ContentBlock::Thinking(tb)) => {
            assert_eq!(tb.thinking, "step one step two");
            assert_eq!(tb.signature.as_deref(), Some("sig-abc"));
        }
        other => panic!("expected thinking block, got {other:?}"),
    }
}

const SSE_MID_STREAM_INLINE_ERROR: &str = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_04\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-5\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\n\
\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\
\n\
event: error\n\
data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"please retry\"}}\n\
\n";

#[tokio::test]
async fn stream_collect_returns_api_error_for_inline_error_event() {
    let result = collect_sse(SSE_MID_STREAM_INLINE_ERROR).await;
    let err = match result {
        Ok(msg) => {
            panic!("expected collect() to surface the inline error event as Err, got Ok({msg:?})")
        }
        Err(e) => e,
    };

    match err {
        clauders::Error::Api(api_err) => {
            assert_eq!(api_err.status, http::StatusCode::OK);
            assert_eq!(api_err.body.kind, clauders::ErrorType::OverloadedError);
            assert_eq!(api_err.body.message, "please retry");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
