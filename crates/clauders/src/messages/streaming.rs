//! SSE streaming wrapper for the Messages API.
//!
//! Exists as a separate module so streaming types are scoped separately
//! from the non-streaming surface.
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
//! - Assembling a [`Message`] from events — the accumulation rules live in
//!   [`crate::messages::MessageAccumulator`]; [`MessageStream::collect`] is
//!   a thin wrapper over it.
//!
//! Entry points: [`MessageStream`] (via `MessagesResource::stream`) and
//! [`StreamEvent`].

use std::pin::Pin;
use std::task::{Context, Poll};

use eventsource_stream::{Event, Eventsource};
use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::error::{ApiError, ApiErrorBody, Error};
use crate::messages::accumulator::MessageAccumulator;
use crate::messages::content::ContentBlock;
use crate::messages::response::{
    Container, Message, OutputTokensDetails, ServerToolUse, StopDetails,
};
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
///
/// # Non-exhaustive matching
///
/// `StreamEvent` is `#[non_exhaustive]`, so a future SDK release can add
/// event types without breaking downstream builds. The consequence for
/// callers: even a `match` that already covers every variant that exists
/// today must still carry a wildcard arm outside this crate, or it fails
/// to compile with `E0004`.
///
/// ```compile_fail
/// use clauders::messages::StreamEvent;
/// let ev: StreamEvent = serde_json::from_str(r#"{"type":"message_stop"}"#).unwrap();
/// let _ = match ev {
///     StreamEvent::MessageStart { .. } => "message_start",
///     StreamEvent::ContentBlockStart { .. } => "content_block_start",
///     StreamEvent::ContentBlockDelta { .. } => "content_block_delta",
///     StreamEvent::ContentBlockStop { .. } => "content_block_stop",
///     StreamEvent::MessageDelta { .. } => "message_delta",
///     StreamEvent::MessageStop => "message_stop",
///     StreamEvent::Ping => "ping",
///     StreamEvent::Error { .. } => "error",
///     StreamEvent::Unknown(_) => "unknown",
///     // No `_` arm: every variant above is covered, but `StreamEvent` is
///     // `#[non_exhaustive]`, so this still fails to compile with E0004.
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
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
    /// An event type this SDK release does not recognize.
    ///
    /// The Anthropic streaming API may add event types at any time, and
    /// documents that clients should handle unrecognized ones gracefully.
    /// The raw JSON object is retained here and the stream continues.
    #[serde(untagged)]
    Unknown(serde_json::Value),
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
#[non_exhaustive]
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
    InputJsonDelta {
        /// The partial JSON string to append to the tool input buffer.
        partial_json: String,
    },
    /// An incremental citation attached to a text block.
    CitationsDelta {
        /// The citation to append to the block's `citations`.
        citation: crate::messages::content::citation::TextCitation,
    },
    /// A delta kind this SDK release does not recognize.
    ///
    /// The Anthropic API may add delta kinds at any time. Rather than
    /// failing the surrounding event, the raw JSON object is retained here
    /// so it can be inspected or logged. Callers that only handle the
    /// modelled kinds may ignore this variant.
    #[serde(untagged)]
    Unknown(serde_json::Value),
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
    /// Structured refusal diagnostic carried on the terminal delta, when present.
    #[serde(default)]
    pub stop_details: Option<StopDetails>,
    /// Container metadata carried on the delta, when present. Decoded for wire
    /// completeness; the accumulator does not fold it (matches the pinned SDKs).
    #[serde(default)]
    pub container: Option<Container>,
}

/// Output-token count carried by [`StreamEvent::MessageDelta`].
///
/// The value reflects the **total** output tokens generated up to this
/// event, not an incremental count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct UsageDelta {
    /// Cumulative input tokens, when the delta reports them.
    #[serde(default)]
    pub input_tokens: Option<u32>,
    /// Cumulative cache-creation input tokens, when reported.
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u32>,
    /// Cumulative cache-read input tokens, when reported.
    #[serde(default)]
    pub cache_read_input_tokens: Option<u32>,
    /// Total output tokens generated so far.
    pub output_tokens: u32,
    /// Read-only decomposition of `output_tokens`, when reported. Decoded for
    /// wire completeness; the accumulator does not fold it (see `accumulator.rs`).
    #[serde(default)]
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// Cumulative server-tool invocation counts, when reported.
    #[serde(default)]
    pub server_tool_use: Option<ServerToolUse>,
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
    /// Every event is folded into a [`MessageAccumulator`]; an inline
    /// `event: error` is intercepted here and returned as [`Error::Api`]
    /// rather than reaching the accumulator. Stops at stream exhaustion.
    ///
    /// Drive a [`MessageAccumulator`] directly instead when the caller needs
    /// to observe events as they arrive and still end up with the assembled
    /// message.
    ///
    /// # Errors
    ///
    /// Returns the first [`Error`] encountered while polling the stream, an
    /// [`Error::Api`] for an inline error event, [`Error::Stream`] if the
    /// stream is truncated or violates the content-block index invariant,
    /// and [`Error::Serde`] if a tool call's accumulated JSON arguments
    /// fail to parse.
    pub async fn collect(mut self) -> Result<Message, Error> {
        let mut accumulator = MessageAccumulator::new();

        loop {
            let item = std::future::poll_fn(|cx| Pin::new(&mut self).poll_next(cx)).await;
            let event = match item {
                None => break,
                Some(Ok(e)) => e,
                Some(Err(e)) => return Err(e),
            };

            if let StreamEvent::Error { error } = event {
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

            accumulator.accumulate(&event)?;
        }

        accumulator.finish()
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

/// Event-type tags this SDK models as first-class [`StreamEvent`] variants.
///
/// `StreamEvent`'s untagged `Unknown` fallback exists so a genuinely
/// unrecognized event type degrades gracefully. But the same fallback also
/// absorbs a *known* event type whose payload fails to satisfy its variant
/// (e.g. an `error` event missing `message`), which would otherwise decode
/// silently as `Unknown` instead of raising a decode error. This list lets
/// [`parse_sse_event`] tell the two cases apart after the fact.
///
/// Kept in sync with `StreamEvent`'s variants by the `event_tag` exhaustive
/// match in this module's test suite: adding a `StreamEvent` variant
/// without a matching arm there fails to compile, and `--all-targets` is
/// part of this crate's Definition of Done.
const KNOWN_EVENT_TAGS: &[&str] = &[
    "message_start",
    "content_block_start",
    "content_block_delta",
    "content_block_stop",
    "message_delta",
    "message_stop",
    "ping",
    "error",
];

fn parse_sse_event(ev: &Event) -> Result<StreamEvent, Error> {
    let parsed = serde_json::from_str::<StreamEvent>(&ev.data).map_err(|e| Error::Serde {
        context: "StreamEvent",
        source: e,
    })?;

    if let StreamEvent::Unknown(ref value) = parsed {
        // A payload that is not a JSON object, or an object with no string
        // `type` field, is a malformed SSE frame rather than a future event
        // — the Anthropic API always tags events by a string `type`, so
        // there is no "genuinely unknown event" reading of these shapes.
        let Some(object) = value.as_object() else {
            return Err(Error::Serde {
                context: "StreamEvent",
                source: <serde_json::Error as serde::de::Error>::custom(
                    "event payload is not a JSON object",
                ),
            });
        };
        let Some(tag) = object.get("type").and_then(serde_json::Value::as_str) else {
            return Err(Error::Serde {
                context: "StreamEvent",
                source: <serde_json::Error as serde::de::Error>::custom(
                    "event payload has no string \"type\" field",
                ),
            });
        };
        if KNOWN_EVENT_TAGS.contains(&tag) {
            return Err(Error::Serde {
                context: "StreamEvent",
                source: <serde_json::Error as serde::de::Error>::custom(format!(
                    "event type \"{tag}\" is modelled but its payload did not satisfy the shape of that variant"
                )),
            });
        }
    }

    Ok(parsed)
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

    /// Wire `"type"` tag for a modelled [`StreamEvent`] variant, or `None`
    /// for [`StreamEvent::Unknown`].
    ///
    /// The match is exhaustive with no wildcard arm: adding a new
    /// `StreamEvent` variant fails to compile here until its tag is
    /// supplied. That failure is what
    /// `known_event_tags_cover_every_modelled_variant` below relies on to
    /// keep [`KNOWN_EVENT_TAGS`] from silently drifting out of sync with
    /// the enum as future event types are modelled.
    fn event_tag(event: &StreamEvent) -> Option<&'static str> {
        match event {
            StreamEvent::MessageStart { .. } => Some("message_start"),
            StreamEvent::ContentBlockStart { .. } => Some("content_block_start"),
            StreamEvent::ContentBlockDelta { .. } => Some("content_block_delta"),
            StreamEvent::ContentBlockStop { .. } => Some("content_block_stop"),
            StreamEvent::MessageDelta { .. } => Some("message_delta"),
            StreamEvent::MessageStop => Some("message_stop"),
            StreamEvent::Ping => Some("ping"),
            StreamEvent::Error { .. } => Some("error"),
            StreamEvent::Unknown(_) => None,
        }
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
    fn parse_unknown_type_yields_unknown_event_with_payload() {
        let ev = make_event(r#"{"type":"unknown_future_event","data":"x"}"#);
        let parsed = parse_sse_event(&ev).unwrap();
        match parsed {
            StreamEvent::Unknown(v) => {
                assert_eq!(v["type"], "unknown_future_event");
                assert_eq!(v["data"], "x");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn known_event_types_still_parse_with_unknown_arm_present() {
        let ev = make_event(r#"{"type":"message_stop"}"#);
        assert!(matches!(
            parse_sse_event(&ev).unwrap(),
            StreamEvent::MessageStop
        ));
    }

    #[test]
    fn parse_malformed_json_returns_serde_error() {
        let ev = make_event("not json at all");
        assert!(parse_sse_event(&ev).is_err());
    }

    #[test]
    fn parse_malformed_known_error_event_yields_serde_error() {
        // The "error" tag is modelled (`StreamEvent::Error`), but this
        // payload's `error` object is missing the required `message` field.
        // A modelled tag whose payload does not satisfy its variant must
        // raise a decode error rather than silently degrade to `Unknown`,
        // since a swallowed inline API error breaks stream termination.
        let ev = make_event(r#"{"type":"error","error":{"type":"overloaded_error"}}"#);
        let err = parse_sse_event(&ev).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Serde {
                    context: "StreamEvent",
                    ..
                }
            ),
            "expected Error::Serde, got {err:?}"
        );
    }

    #[test]
    fn parse_malformed_known_message_start_event_yields_serde_error() {
        // The "message_start" tag is modelled, but this payload has no
        // `message` field at all — the same fall-through hazard as the
        // malformed `error` case above, for a different modelled variant.
        let ev = make_event(r#"{"type":"message_start"}"#);
        let err = parse_sse_event(&ev).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Serde {
                    context: "StreamEvent",
                    ..
                }
            ),
            "expected Error::Serde, got {err:?}"
        );
    }

    #[test]
    fn parse_non_object_string_payload_returns_serde_error() {
        // A bare JSON string is not a valid SSE event frame at all — it is
        // malformed input, not a future event type, so it must not decode
        // to `StreamEvent::Unknown`.
        let ev = make_event(r#""just_a_string""#);
        let err = parse_sse_event(&ev).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Serde {
                    context: "StreamEvent",
                    ..
                }
            ),
            "expected Error::Serde, got {err:?}"
        );
    }

    #[test]
    fn parse_non_object_number_payload_returns_serde_error() {
        let ev = make_event("42");
        let err = parse_sse_event(&ev).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Serde {
                    context: "StreamEvent",
                    ..
                }
            ),
            "expected Error::Serde, got {err:?}"
        );
    }

    #[test]
    fn parse_null_payload_returns_serde_error() {
        let ev = make_event("null");
        let err = parse_sse_event(&ev).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Serde {
                    context: "StreamEvent",
                    ..
                }
            ),
            "expected Error::Serde, got {err:?}"
        );
    }

    #[test]
    fn known_event_tags_cover_every_modelled_variant() {
        // One minimal, validly-shaped fixture per modelled `StreamEvent`
        // variant. `event_tag`'s match is exhaustive over `StreamEvent`, so
        // this loop, combined with that exhaustiveness, is what keeps
        // `KNOWN_EVENT_TAGS` from drifting: a `StreamEvent` variant added
        // without a matching `event_tag` arm fails to compile, and a
        // variant added to `event_tag` but never wired into
        // `KNOWN_EVENT_TAGS` fails this assertion instead of passing silently.
        let fixtures = [
            r#"{"type":"message_start","message":{"id":"msg_0","type":"message","role":"assistant","model":"m","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":0,"output_tokens":0}}}"#,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":""}}"#,
            r#"{"type":"content_block_stop","index":0}"#,
            r#"{"type":"message_delta","delta":{"stop_reason":null,"stop_sequence":null},"usage":{"output_tokens":0}}"#,
            r#"{"type":"message_stop"}"#,
            r#"{"type":"ping"}"#,
            r#"{"type":"error","error":{"type":"overloaded_error","message":""}}"#,
        ];

        for fixture in fixtures {
            let event: StreamEvent = serde_json::from_str(fixture).unwrap();
            let Some(tag) = event_tag(&event) else {
                panic!(
                    "fixture {fixture} decoded to StreamEvent::Unknown, expected a modelled variant"
                );
            };
            assert!(
                KNOWN_EVENT_TAGS.contains(&tag),
                "tag {tag:?} (from {fixture}) is missing from KNOWN_EVENT_TAGS"
            );
        }
    }

    #[test]
    fn parse_object_without_string_type_field_returns_serde_error() {
        // An object with no string `type` field cannot be a genuinely
        // unrecognized future event either — the API always tags events by
        // a string `type`, so this is a malformed frame.
        let ev = make_event(r#"{"foo":1}"#);
        let err = parse_sse_event(&ev).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Serde {
                    context: "StreamEvent",
                    ..
                }
            ),
            "expected Error::Serde, got {err:?}"
        );
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

    #[test]
    fn content_delta_unknown_variant_retains_payload() {
        // `citations_delta` used to be this fixture's stand-in for an
        // unrecognized tag; it is now a modelled variant (`CitationsDelta`),
        // so this uses a tag that stays unmodeled instead.
        let json = r#"{"type":"future_delta","payload":{"cited_text":"x"}}"#;
        let d: ContentDelta = serde_json::from_str(json).unwrap();
        match d {
            ContentDelta::Unknown(v) => {
                assert_eq!(v["type"], "future_delta");
                assert_eq!(v["payload"]["cited_text"], "x");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn content_delta_known_variants_still_decode_with_unknown_arm_present() {
        let d: ContentDelta = serde_json::from_str(r#"{"type":"text_delta","text":"hi"}"#).unwrap();
        assert!(matches!(d, ContentDelta::TextDelta { text } if text == "hi"));
    }

    #[test]
    fn content_delta_missing_required_field_falls_through_to_unknown() {
        // A known tag whose payload does not satisfy the variant is absorbed by
        // the fallback rather than raising a decode error. Pinned so this
        // behavior is deliberate, not an oversight.
        let d: ContentDelta = serde_json::from_str(r#"{"type":"text_delta"}"#).unwrap();
        assert!(matches!(d, ContentDelta::Unknown(_)));
    }

    #[test]
    fn citations_delta_decodes() {
        let json = r#"{"type":"citations_delta","citation":
            {"type":"page_location","cited_text":"x","document_index":0,
             "start_page_number":1,"end_page_number":2}}"#;
        let d: ContentDelta = serde_json::from_str(json).unwrap();
        assert!(matches!(d, ContentDelta::CitationsDelta { .. }));
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

    #[test]
    fn message_meta_delta_decodes_stop_details_and_container() {
        use crate::messages::response::RefusalCategory;

        let j = r#"{
            "stop_reason": "refusal",
            "stop_sequence": null,
            "stop_details": {"type":"refusal","category":"cyber","explanation":"x"},
            "container": {"id":"c9","expires_at":"2026-07-22T00:00:00Z"}
        }"#;
        let d: MessageMetaDelta = serde_json::from_str(j).unwrap();
        assert_eq!(
            d.stop_details.map(|s| s.category),
            Some(Some(RefusalCategory::Cyber))
        );
        assert_eq!(d.container.map(|c| c.id), Some("c9".to_owned()));
    }

    #[test]
    fn usage_delta_decodes_input_side_counters() {
        let j = r#"{
            "input_tokens": 100,
            "cache_creation_input_tokens": 10,
            "cache_read_input_tokens": 5,
            "output_tokens": 42,
            "output_tokens_details": {"thinking_tokens": 3},
            "server_tool_use": {"web_search_requests": 1, "web_fetch_requests": 0}
        }"#;
        let d: UsageDelta = serde_json::from_str(j).unwrap();
        assert_eq!(d.input_tokens, Some(100));
        assert_eq!(d.cache_creation_input_tokens, Some(10));
        assert_eq!(d.cache_read_input_tokens, Some(5));
        assert_eq!(d.output_tokens, 42);
        assert_eq!(d.output_tokens_details.map(|o| o.thinking_tokens), Some(3));
        assert_eq!(d.server_tool_use.map(|s| s.web_search_requests), Some(1));
    }

    #[test]
    fn usage_delta_input_side_default_to_none() {
        let d: UsageDelta = serde_json::from_str(r#"{"output_tokens":7}"#).unwrap();
        assert_eq!(d.input_tokens, None);
        assert_eq!(d.output_tokens, 7);
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
