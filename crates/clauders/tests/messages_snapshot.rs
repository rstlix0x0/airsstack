//! Snapshot tests for serialized `MessageRequest` bodies.
//!
//! Each snapshot locks the exact JSON wire format produced by the builder.
//! When the serialization shape changes intentionally, run with
//! `INSTA_UPDATE=always cargo test` to regenerate, then review the diff.

#![cfg(feature = "messages")]
#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
)]

use clauders::messages::MessageRequest;
use clauders::types::{MaxTokens, ModelId, StopSequence, SystemPrompt, Temperature};

#[test]
fn snapshot_minimal_request() {
    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(64).unwrap())
        .add_user_text("Hi")
        .build();

    let json = serde_json::to_string_pretty(&req).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn snapshot_request_with_optional_params() {
    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(MaxTokens::new(256).unwrap())
        .system(SystemPrompt::text("You are terse."))
        .temperature(Temperature::new(0.7).unwrap())
        .stop_sequences(vec![StopSequence::new("STOP").unwrap()])
        .add_user_text("Hello")
        .add_assistant_text("Hi there.")
        .build();

    let json = serde_json::to_string_pretty(&req).unwrap();
    insta::assert_snapshot!(json);
}
