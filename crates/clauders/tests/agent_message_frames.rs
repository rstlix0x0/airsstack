//! Integration: the public Message decoder surfaces the metadata the SDK
//! exposes, using frames captured from a live `claude` stream-json run.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
)]

use clauders::agent::Message;

#[test]
fn public_decoder_surfaces_result_and_assistant_metadata() {
    // captured: claude -p "say hi" --output-format stream-json --verbose
    let result = r#"{"type":"result","subtype":"success","is_error":false,"result":"Hi.",
      "num_turns":1,"session_id":"s1","duration_ms":1971,"ttft_ms":1945,"terminal_reason":"completed",
      "uuid":"24e65d43","permission_denials":[],
      "modelUsage":{"claude-opus-4-8[1m]":{"inputTokens":2,"outputTokens":6}}}"#;
    let Message::Result(r) = serde_json::from_str(result).expect("result") else {
        panic!("expected Result");
    };
    assert_eq!(r.ttft_ms, Some(1945));
    assert!(r.model_usage.contains_key("claude-opus-4-8[1m]"));

    let assistant = r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-opus-4-8",
      "role":"assistant","content":[{"type":"text","text":"Hi."}],"context_management":null},
      "session_id":"s1","request_id":"req_1"}"#;
    let Message::Assistant(a) = serde_json::from_str(assistant).expect("assistant") else {
        panic!("expected Assistant");
    };
    assert_eq!(a.id.as_deref(), Some("msg_1"));
    assert_eq!(a.request_id.as_deref(), Some("req_1"));
    assert!(a.extra.get("context_management").is_some());
}
