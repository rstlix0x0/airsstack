//! SSE streaming wrapper for the Messages API.
//!
//! Exists as a separate module gated behind `messages-streaming` so the
//! `eventsource-stream` dependency is only compiled when the feature is
//! enabled, and so streaming types do not pollute the non-streaming surface.
//!
//! Responsibilities:
//! - Define [`StreamEvent`], the typed union of every SSE event the
//!   Anthropic API emits on a streaming response.
//! - Define [`ContentDelta`], [`MessageMetaDelta`], and [`UsageDelta`] —
//!   sub-types carried by specific event variants.
//! - Define [`MessageStream`], the `Stream<Item = Result<StreamEvent, Error>>`
//!   wrapper that drives SSE parsing and enforces the terminal-on-error rule.
//! - Provide [`MessageStream::collect`], which drains the stream and
//!   assembles the complete [`Message`] from its events.
//!
//! Not responsible for:
//! - Building the HTTP request or choosing the URL — that lives in
//!   `resource.rs` (`MessagesResource::stream`).
//! - Tool-use content deltas are not modelled.
//!
//! Entry points: [`MessageStream`] (via `MessagesResource::stream`) and
//! [`StreamEvent`].

use std::pin::Pin;
use std::task::{Context, Poll};

use eventsource_stream::{Event, Eventsource};
use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::error::{ApiError, ApiErrorBody, Error};
use crate::messages::content::{ContentBlock, TextBlock};
use crate::messages::response::{Message, Usage};
use crate::transport::BodyStream;
use crate::types::StopSequence;

use super::response::StopReason;

/// Every event the Anthropic streaming API can emit on a `POST /v1/messages`
/// response with `"stream": true`.
///
/// The `"type"` field in the SSE data JSON acts as the serde discriminant.
///
/// # Examples
///
/// ```
/// use clauders::messages::StreamEvent;
/// let json = r#"{"type":"message_stop"}"#;
/// let ev: StreamEvent = serde_json::from_str(json).unwrap();
/// assert!(matches!(ev, StreamEvent::MessageStop));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// The response message has started; carries an initial [`Message`]
    /// shell with empty content and token counters.
    MessageStart {
        /// Initial message shell.
        message: Message,
    },
    /// A new content block has started at `index`.
    ContentBlockStart {
        /// Zero-based position in the content array.
        index: u32,
        /// The initial content block (usually an empty text block).
        content_block: ContentBlock,
    },
    /// An incremental delta to the content block at `index`.
    ContentBlockDelta {
        /// Zero-based position in the content array.
        index: u32,
        /// The delta payload.
        delta: ContentDelta,
    },
    /// The content block at `index` is complete.
    ContentBlockStop {
        /// Zero-based position in the content array.
        index: u32,
    },
    /// Final metadata for the message: stop reason, stop sequence, and
    /// accumulated output-token count.
    MessageDelta {
        /// Stop-reason and stop-sequence updates.
        delta: MessageMetaDelta,
        /// Final token-usage counters.
        usage: UsageDelta,
    },
    /// The message is fully streamed; no more events follow.
    MessageStop,
    /// Keepalive event; carries no data.
    Ping,
    /// An inline error event; the stream is terminal after this.
    Error {
        /// The error payload from the API.
        error: ApiErrorBody,
    },
}

/// Incremental delta for a content block.
///
/// # Examples
///
/// ```
/// use clauders::messages::ContentDelta;
/// let json = r#"{"type":"text_delta","text":"hello"}"#;
/// let d: ContentDelta = serde_json::from_str(json).unwrap();
/// assert!(matches!(d, ContentDelta::TextDelta { .. }));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    /// An incremental text fragment.
    TextDelta {
        /// The text fragment to append.
        text: String,
    },
    /// An incremental extended-thinking fragment.
    ThinkingDelta {
        /// The thinking fragment to append.
        thinking: String,
    },
    /// A cryptographic signature for extended-thinking verification.
    SignatureDelta {
        /// The signature fragment.
        signature: String,
    },
    /// An incremental fragment of the JSON arguments being assembled for a
    /// tool invocation.
    #[cfg(feature = "messages-tools")]
    InputJsonDelta {
        /// The partial JSON string to append to the tool input buffer.
        partial_json: String,
    },
}

/// Stop-reason and stop-sequence fields carried by [`StreamEvent::MessageDelta`].
///
/// Both fields may be absent mid-stream; `stop_reason` is set in the final
/// `message_delta` event when generation ends.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct MessageMetaDelta {
    /// Why the model stopped generating, once known.
    pub stop_reason: Option<StopReason>,
    /// Which stop sequence triggered the stop, if any.
    pub stop_sequence: Option<StopSequence>,
}

/// Output-token count carried by [`StreamEvent::MessageDelta`].
///
/// The value reflects the **total** output tokens generated up to this
/// event, not an incremental count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct UsageDelta {
    /// Total output tokens generated so far.
    pub output_tokens: u32,
}

pin_project! {
    /// Asynchronous stream of [`StreamEvent`] items for a streaming
    /// Messages API response.
    ///
    /// Obtain via [`crate::messages::MessagesResource::stream`].
    ///
    /// The stream is **terminal on an inline error event**: once a
    /// [`StreamEvent::Error`] is delivered, the next poll returns `None`.
    /// SSE transport failures map to [`Error::Stream`].
    ///
    /// Use [`MessageStream::collect`] to drain the stream and assemble
    /// the final [`Message`].
    #[must_use = "streams do nothing unless polled"]
    pub struct MessageStream {
        #[pin]
        inner: eventsource_stream::EventStream<BodyStream>,
        terminated: bool,
    }
}

impl MessageStream {
    /// Wrap a raw body stream as a `MessageStream`.
    pub(crate) fn new(body: BodyStream) -> Self {
        Self {
            inner: body.eventsource(),
            terminated: false,
        }
    }

    /// Drain the stream and assemble the final [`Message`].
    ///
    /// Consumes `message_start`, `content_block_*`, and `message_delta`
    /// events to build the complete message. Stops at `message_stop` or
    /// stream exhaustion.
    ///
    /// # Errors
    ///
    /// Returns the first [`Error`] encountered while polling the stream.
    /// An inline `event: error` becomes [`Error::Api`] and the stream is
    /// consumed as terminal.
    pub async fn collect(mut self) -> Result<Message, Error> {
        let mut accumulated: Option<Message> = None;

        loop {
            let item = std::future::poll_fn(|cx| Pin::new(&mut self).poll_next(cx)).await;
            let event = match item {
                None => break,
                Some(Ok(e)) => e,
                Some(Err(e)) => return Err(e),
            };

            match event {
                StreamEvent::MessageStart { message } => {
                    accumulated = Some(message);
                }
                StreamEvent::ContentBlockStart {
                    index,
                    content_block,
                } => {
                    if let Some(ref mut m) = accumulated {
                        let idx = index as usize;
                        while m.content.len() <= idx {
                            m.content.push(ContentBlock::Text(TextBlock::new("")));
                        }
                        m.content[idx] = content_block;
                    }
                }
                StreamEvent::ContentBlockDelta { index, delta } => {
                    if let Some(ref mut m) = accumulated {
                        let idx = index as usize;
                        if let (Some(ContentBlock::Text(tb)), ContentDelta::TextDelta { text }) =
                            (m.content.get_mut(idx), delta)
                        {
                            tb.text.push_str(&text);
                        }
                    }
                }
                StreamEvent::MessageDelta { delta, usage } => {
                    if let Some(ref mut m) = accumulated {
                        if delta.stop_reason.is_some() {
                            m.stop_reason = delta.stop_reason;
                        }
                        if delta.stop_sequence.is_some() {
                            m.stop_sequence = delta.stop_sequence;
                        }
                        m.usage = Usage {
                            input_tokens: m.usage.input_tokens,
                            output_tokens: usage.output_tokens,
                            #[cfg(feature = "messages-caching")]
                            cache_creation_input_tokens: m.usage.cache_creation_input_tokens,
                            #[cfg(feature = "messages-caching")]
                            cache_read_input_tokens: m.usage.cache_read_input_tokens,
                            #[cfg(feature = "messages-caching")]
                            cache_creation: m.usage.cache_creation,
                        };
                    }
                }
                StreamEvent::Error { error } => {
                    return Err(Error::Api(ApiError {
                        // An inline stream-level error has no HTTP status code;
                        // use 200 as the nominal status since the HTTP layer
                        // already returned success when the stream started.
                        status: http::StatusCode::OK,
                        body: error,
                        request_id: None,
                        organization_id: None,
                        retry_after: None,
                    }));
                }
                StreamEvent::MessageStop
                | StreamEvent::ContentBlockStop { .. }
                | StreamEvent::Ping => {}
            }
        }

        accumulated.ok_or_else(|| Error::Stream("stream ended before message_start event".into()))
    }
}

impl Stream for MessageStream {
    type Item = Result<StreamEvent, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if *this.terminated {
            return Poll::Ready(None);
        }

        match this.inner.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                *this.terminated = true;
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(e))) => {
                *this.terminated = true;
                Poll::Ready(Some(Err(Error::Stream(e.to_string()))))
            }
            Poll::Ready(Some(Ok(event))) => match parse_sse_event(&event) {
                Ok(parsed) => {
                    if matches!(parsed, StreamEvent::Error { .. }) {
                        *this.terminated = true;
                    }
                    Poll::Ready(Some(Ok(parsed)))
                }
                Err(e) => {
                    *this.terminated = true;
                    Poll::Ready(Some(Err(e)))
                }
            },
        }
    }
}

fn parse_sse_event(ev: &Event) -> Result<StreamEvent, Error> {
    serde_json::from_str::<StreamEvent>(&ev.data).map_err(|e| Error::Serde {
        context: "StreamEvent",
        source: e,
    })
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::panic,
        reason = "test-only panics on wrong-variant matches; a panic is the intended failure signal"
    )]

    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use bytes::Bytes;
    use eventsource_stream::Event;
    use futures_core::Stream as FutStream;

    use super::*;

    // ── helpers ────────────────────────────────────────────────────────────────

    fn make_event(data: &str) -> Event {
        Event {
            data: data.to_owned(),
            ..Event::default()
        }
    }

    /// Single-shot in-memory byte stream for unit tests.
    struct ByteChunk(VecDeque<Bytes>);

    impl FutStream for ByteChunk {
        type Item = Result<Bytes, crate::error::TransportError>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.0.pop_front().map(Ok))
        }
    }

    fn body_from_static(bytes: &'static [u8]) -> BodyStream {
        Box::pin(ByteChunk(VecDeque::from([Bytes::from_static(bytes)])))
    }

    fn body_from_owned(s: &str) -> BodyStream {
        Box::pin(ByteChunk(VecDeque::from([Bytes::copy_from_slice(
            s.as_bytes(),
        )])))
    }

    // ── parse_sse_event ────────────────────────────────────────────────────────

    #[test]
    fn parse_message_stop() {
        let ev = make_event(r#"{"type":"message_stop"}"#);
        let parsed = parse_sse_event(&ev).unwrap();
        assert!(matches!(parsed, StreamEvent::MessageStop));
    }

    #[test]
    fn parse_ping() {
        let ev = make_event(r#"{"type":"ping"}"#);
        let parsed = parse_sse_event(&ev).unwrap();
        assert!(matches!(parsed, StreamEvent::Ping));
    }

    #[test]
    fn parse_content_block_delta_text() {
        let ev = make_event(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hi"}}"#,
        );
        let parsed = parse_sse_event(&ev).unwrap();
        match parsed {
            StreamEvent::ContentBlockDelta {
                index,
                delta: ContentDelta::TextDelta { text },
            } => {
                assert_eq!(index, 0);
                assert_eq!(text, "hi");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn parse_error_event() {
        let ev = make_event(
            r#"{"type":"error","error":{"type":"overloaded_error","message":"please retry"}}"#,
        );
        let parsed = parse_sse_event(&ev).unwrap();
        assert!(matches!(parsed, StreamEvent::Error { .. }));
    }

    #[test]
    fn parse_unknown_type_returns_serde_error() {
        let ev = make_event(r#"{"type":"unknown_future_event","data":"x"}"#);
        assert!(parse_sse_event(&ev).is_err());
    }

    #[test]
    fn parse_malformed_json_returns_serde_error() {
        let ev = make_event("not json at all");
        assert!(parse_sse_event(&ev).is_err());
    }

    // ── ContentDelta serde ─────────────────────────────────────────────────────

    #[test]
    fn content_delta_thinking_variant() {
        let json = r#"{"type":"thinking_delta","thinking":"deep thought"}"#;
        let d: ContentDelta = serde_json::from_str(json).unwrap();
        match d {
            ContentDelta::ThinkingDelta { thinking } => assert_eq!(thinking, "deep thought"),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn content_delta_signature_variant() {
        let json = r#"{"type":"signature_delta","signature":"abc123"}"#;
        let d: ContentDelta = serde_json::from_str(json).unwrap();
        match d {
            ContentDelta::SignatureDelta { signature } => assert_eq!(signature, "abc123"),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ── MessageMetaDelta + UsageDelta serde ────────────────────────────────────

    #[test]
    fn message_delta_event_parses_stop_reason_and_usage() {
        let json = r#"{
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn", "stop_sequence": null},
            "usage": {"output_tokens": 42}
        }"#;
        let ev: StreamEvent = serde_json::from_str(json).unwrap();
        match ev {
            StreamEvent::MessageDelta { delta, usage } => {
                assert_eq!(delta.stop_reason, Some(StopReason::EndTurn));
                assert!(delta.stop_sequence.is_none());
                assert_eq!(usage.output_tokens, 42);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ── Stream terminates after Error event ────────────────────────────────────

    #[tokio::test]
    async fn stream_terminates_after_error_event() {
        let sse: &[u8] = b"event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"busy\"}}\n\n";
        let body = body_from_static(sse);
        let mut stream = MessageStream::new(body);

        // First poll: the Error event.
        let first = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        assert!(
            matches!(first, Some(Ok(StreamEvent::Error { .. }))),
            "expected Error event, got {first:?}"
        );

        // Second poll: terminal None.
        let second = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        assert!(
            second.is_none(),
            "expected terminal None after error event, got {second:?}"
        );
    }

    // ── collect: truncated stream before message_start ────────────────────────

    #[tokio::test]
    async fn collect_returns_stream_error_when_no_message_start() {
        // A stream that ends immediately without emitting any events exercises
        // the truncation path: the accumulated message is None and collect
        // must surface Error::Stream, not Error::InvalidRequest, because the
        // failure is a server-side protocol truncation, not a client request error.
        let body = body_from_owned("");
        let result = MessageStream::new(body).collect().await;
        assert!(
            matches!(result, Err(Error::Stream(_))),
            "expected Error::Stream for truncated stream, got {result:?}"
        );
    }

    // ── collect assembles message text ─────────────────────────────────────────

    #[tokio::test]
    async fn collect_assembles_text_from_deltas() {
        let sse = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_01\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-5\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":5,\"output_tokens\":0}}}\n",
            "\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n",
            "\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"foo\"}}\n",
            "\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"bar\"}}\n",
            "\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n",
            "\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":2}}\n",
            "\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n",
            "\n",
        );

        let body = body_from_owned(sse);
        let msg = MessageStream::new(body).collect().await.unwrap();

        assert_eq!(msg.usage.output_tokens, 2);
        match msg.content.first() {
            Some(ContentBlock::Text(tb)) => assert_eq!(tb.text, "foobar"),
            other => panic!("expected text block, got {other:?}"),
        }
    }
}
