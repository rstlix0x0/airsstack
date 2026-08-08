//! Incremental assembly of a [`Message`] from a sequence of streaming events.
//!
//! Exists as its own module so the accumulation rules are unit-testable
//! without constructing an SSE byte stream, and so callers driving their own
//! event loop can reuse them.
//!
//! Responsibilities:
//! - Define [`MessageAccumulator`], the state machine that folds
//!   [`StreamEvent`] values into a complete [`Message`].
//!
//! Not responsible for:
//! - SSE parsing or transport — that lives in `streaming.rs`.
//! - Interpreting [`StreamEvent::Error`]; the accumulator treats it as inert
//!   so it stays a pure state machine over well-formed events. Callers
//!   decide what an inline error means.
//!
//! Entry point: [`MessageAccumulator::new`].

use crate::error::Error;
use crate::messages::content::ContentBlock;
use crate::messages::response::Message;
use crate::messages::streaming::{ContentDelta, StreamEvent};

/// Folds streaming events into a complete [`Message`].
///
/// Feed every event to [`accumulate`](MessageAccumulator::accumulate) in
/// arrival order, then call [`finish`](MessageAccumulator::finish) once the
/// stream is drained.
///
/// [`crate::messages::MessageStream::collect`] wraps this type and is the
/// simpler choice when the events themselves are not needed. Drive the
/// accumulator directly when the caller wants to observe events as they
/// arrive *and* end up with the assembled message.
///
/// # Examples
///
/// ```
/// use clauders::messages::MessageAccumulator;
///
/// let start: clauders::messages::StreamEvent = serde_json::from_str(r#"{
///     "type": "message_start",
///     "message": {
///         "id": "msg_01", "type": "message", "role": "assistant",
///         "model": "claude-sonnet-4-5", "content": [],
///         "stop_reason": null, "stop_sequence": null,
///         "usage": {"input_tokens": 1, "output_tokens": 0}
///     }
/// }"#).unwrap();
///
/// let mut acc = MessageAccumulator::new();
/// acc.accumulate(&start).unwrap();
/// let message = acc.finish().unwrap();
/// assert_eq!(message.id.as_str(), "msg_01");
/// ```
#[derive(Clone, Debug)]
pub struct MessageAccumulator {
    snapshot: Option<Message>,
    json_bufs: Vec<String>,
}

impl MessageAccumulator {
    /// Create an empty accumulator awaiting a `message_start` event.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            snapshot: None,
            json_bufs: Vec::new(),
        }
    }

    /// Fold one event into the accumulated message.
    ///
    /// Events that arrive before `message_start` are ignored, as are deltas
    /// addressing a content block that does not exist and deltas whose kind
    /// does not match the block they address.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Stream`] when a `content_block_start` does not
    /// address the next content position, and [`Error::Serde`] when the
    /// JSON arguments accumulated for a tool call fail to parse at
    /// `content_block_stop`.
    ///
    /// Either error leaves the accumulator internally coherent — nothing is
    /// mutated before the error is returned, so no fabricated or
    /// half-written block is left behind. No error is latched: the
    /// accumulator keeps accepting events, and a later
    /// `content_block_start` that does address the current content length
    /// still succeeds.
    ///
    /// Against a real server that is a distinction without a difference,
    /// because indices only ever advance: once a gapped
    /// `content_block_start` is rejected, the content vector's length stops
    /// tracking the server's index, so every subsequent start fails the
    /// same check while every delta and stop addressing those indices
    /// silently no-ops back to `Ok(())`. A caller that logs the error and
    /// keeps feeding events therefore ends up with a truncated message and
    /// exactly one signal for what may be an unbounded number of dropped
    /// blocks.
    pub fn accumulate(&mut self, event: &StreamEvent) -> Result<(), Error> {
        match event {
            // All three official SDKs disagree on what a second
            // message_start should do: TypeScript throws
            // (`MessageStream.ts:562-564`); Python has no guard at all and
            // silently keeps appending the second message's content blocks
            // onto the first message's content list, with no error
            // (`_messages.py:450-464`); Go replaces the whole message
            // struct, discarding everything accumulated so far
            // (`messageutil.go:26-27`). This accumulator follows Go: both
            // the snapshot and `json_bufs` are replaced wholesale, so
            // nothing accumulated under a prior `message_start` — content,
            // buffered tool JSON, or otherwise — survives a second one.
            StreamEvent::MessageStart { message } => {
                self.json_bufs = vec![String::new(); message.content.len()];
                self.snapshot = Some(message.clone());
                Ok(())
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => self.start_block(*index, content_block),
            StreamEvent::ContentBlockDelta { index, delta } => {
                self.apply_delta(*index, delta);
                Ok(())
            }
            StreamEvent::ContentBlockStop { index } => self.finish_block(*index),
            // Python (`_messages.py:504-505`) and TypeScript
            // (`MessageStream.ts:576-577`) both write stop_reason and
            // stop_sequence unconditionally on every message_delta,
            // including overwriting an already-resolved value with null.
            // This accumulator writes them only when the delta actually
            // carries a value, so a stray later message_delta cannot clobber
            // a resolved stop_reason or stop_sequence. This assumes the
            // terminal delta is the one that carries these fields — a
            // real server sending a later, empty message_delta after the
            // resolving one is not something we have verified either way.
            StreamEvent::MessageDelta { delta, usage } => {
                if let Some(snapshot) = self.snapshot.as_mut() {
                    if delta.stop_reason.is_some() {
                        snapshot.stop_reason.clone_from(&delta.stop_reason);
                    }
                    if delta.stop_sequence.is_some() {
                        snapshot.stop_sequence.clone_from(&delta.stop_sequence);
                    }
                    // Mirrors the pinned Python SDK's fold policy
                    // (`_messages.py:503-518`): stop_details is folded when
                    // the delta carries it; container/service_tier/
                    // inference_geo/output_tokens_details are decoded on the
                    // wire types but deliberately not folded, matching that
                    // source.
                    if delta.stop_details.is_some() {
                        snapshot.stop_details.clone_from(&delta.stop_details);
                    }
                    // output_tokens is written unconditionally; the
                    // input-side counters overwrite only when the delta
                    // actually reports them.
                    snapshot.usage.output_tokens = usage.output_tokens;
                    if let Some(v) = usage.input_tokens {
                        snapshot.usage.input_tokens = v;
                    }
                    if usage.cache_creation_input_tokens.is_some() {
                        snapshot.usage.cache_creation_input_tokens =
                            usage.cache_creation_input_tokens;
                    }
                    if usage.cache_read_input_tokens.is_some() {
                        snapshot.usage.cache_read_input_tokens = usage.cache_read_input_tokens;
                    }
                    if usage.server_tool_use.is_some() {
                        snapshot.usage.server_tool_use = usage.server_tool_use;
                    }
                }
                Ok(())
            }
            // An inline error is a caller-facing concern, not an
            // accumulation one: leaving it inert keeps this a pure state
            // machine over well-formed events.
            StreamEvent::MessageStop
            | StreamEvent::Ping
            | StreamEvent::Error { .. }
            | StreamEvent::Unknown(_) => Ok(()),
        }
    }

    /// Append a newly started content block at `index`.
    ///
    /// The API starts content blocks in index order with no gaps: a start
    /// event always addresses the slot immediately after the previous block,
    /// even when deltas and stops for still-open blocks interleave after it.
    /// A violation of that invariant is reported rather than papered over,
    /// because the alternative — padding the content vector — leaves
    /// fabricated blocks indistinguishable from ones the model produced.
    fn start_block(&mut self, index: u32, block: &ContentBlock) -> Result<(), Error> {
        let Some(snapshot) = self.snapshot.as_mut() else {
            return Ok(());
        };
        let idx = index as usize;
        if idx != snapshot.content.len() {
            return Err(Error::Stream(format!(
                "content_block_start index {idx} does not address the next content position {}",
                snapshot.content.len()
            )));
        }
        snapshot.content.push(block.clone());
        self.json_bufs.push(String::new());
        Ok(())
    }

    /// Apply one delta to the content block at `index`.
    ///
    /// A delta addressing a block that does not exist, or whose kind does
    /// not match the block it addresses, is ignored: the server is the
    /// authority on which pairings are valid, and a mismatch is far more
    /// likely to mean this SDK release does not model the block than that
    /// the response is corrupt.
    fn apply_delta(&mut self, index: u32, delta: &ContentDelta) {
        let idx = index as usize;

        if let ContentDelta::InputJsonDelta { partial_json } = delta {
            // Tool arguments stream as raw JSON fragments that are only
            // valid once concatenated, so they are buffered beside the
            // snapshot and parsed in one pass at content_block_stop.
            let addresses_tool_block = matches!(
                self.snapshot.as_ref().and_then(|s| s.content.get(idx)),
                Some(ContentBlock::ToolUse(_) | ContentBlock::ServerToolUse(_))
            );
            if addresses_tool_block && let Some(buffer) = self.json_bufs.get_mut(idx) {
                buffer.push_str(partial_json);
            }
            return;
        }

        let Some(snapshot) = self.snapshot.as_mut() else {
            return;
        };
        let Some(block) = snapshot.content.get_mut(idx) else {
            return;
        };
        match delta {
            ContentDelta::TextDelta { text } => {
                if let ContentBlock::Text(target) = block {
                    target.text.push_str(text);
                }
            }
            ContentDelta::ThinkingDelta { thinking } => {
                if let ContentBlock::Thinking(target) = block {
                    target.thinking.push_str(thinking);
                }
            }
            ContentDelta::SignatureDelta { signature } => {
                // The API emits exactly one signature delta per thinking
                // block, so replacing is equivalent to concatenating onto
                // the empty signature the start event carries.
                if let ContentBlock::Thinking(target) = block {
                    target.signature = Some(signature.clone());
                }
            }
            ContentDelta::CitationsDelta { citation } => {
                if let ContentBlock::Text(target) = block {
                    target
                        .citations
                        .get_or_insert_with(Vec::new)
                        .push(citation.clone());
                }
            }
            ContentDelta::InputJsonDelta { .. } | ContentDelta::Unknown(_) => {}
        }
    }

    /// Finalize the content block at `index`.
    ///
    /// Parses the JSON arguments accumulated for a tool call, if any. An
    /// empty buffer is not an error: a zero-argument tool call produces no
    /// fragments beyond the empty one the API opens the block with, and the
    /// input the start event carried is already correct.
    ///
    /// A block that never receives a `content_block_stop` — a truncated
    /// stream, or a caller reading the accumulator mid-flight — never
    /// reaches this parse step, so its `input` stays at whatever the
    /// `content_block_start` event carried (`{}`). Python's eager
    /// per-delta parse (`jiter.from_json(json_buf, partial_mode=True)`,
    /// `_messages.py:479-480`) and TypeScript's lazy parse-on-read
    /// (`partialParse`, `internal/message-stream-utils.ts:21-27`) both
    /// instead salvage whatever complete key-value pairs a truncated
    /// buffer holds — `{"a": 1,` becomes `{"a": 1}` in both — and drop the
    /// rest. This accumulator does not attempt that salvage: a block's
    /// input is either the result of a full parse of its buffer at stop,
    /// or exactly what the server first described in the start event.
    fn finish_block(&mut self, index: u32) -> Result<(), Error> {
        let idx = index as usize;
        let Some(buffer) = self.json_bufs.get_mut(idx) else {
            return Ok(());
        };
        if buffer.is_empty() {
            return Ok(());
        }
        let parsed: serde_json::Value =
            serde_json::from_str(buffer).map_err(|source| Error::Serde {
                context: "ToolUseBlock.input",
                source,
            })?;
        buffer.clear();

        match self.snapshot.as_mut().and_then(|s| s.content.get_mut(idx)) {
            Some(ContentBlock::ToolUse(target)) => target.input = parsed,
            Some(ContentBlock::ServerToolUse(target)) => target.input = parsed,
            _ => {}
        }
        Ok(())
    }

    /// Consume the accumulator and return the assembled message.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Stream`] when no `message_start` event was ever
    /// accumulated — the stream was truncated before it began.
    pub fn finish(self) -> Result<Message, Error> {
        self.snapshot
            .ok_or_else(|| Error::Stream("stream ended before message_start event".into()))
    }
}

impl Default for MessageAccumulator {
    fn default() -> Self {
        Self::new()
    }
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

    use super::*;
    use crate::messages::response::{RefusalCategory, StopReason};
    use crate::types::StopSequence;

    // ── fixtures ───────────────────────────────────────────────────────────

    fn event(json: &str) -> StreamEvent {
        serde_json::from_str(json).unwrap()
    }

    fn message_start() -> StreamEvent {
        event(
            r#"{"type":"message_start","message":{
                "id":"msg_01","type":"message","role":"assistant",
                "model":"claude-sonnet-4-5","content":[],
                "stop_reason":null,"stop_sequence":null,
                "usage":{"input_tokens":5,"output_tokens":0}}}"#,
        )
    }

    // ── message_start / finish ─────────────────────────────────────────────

    #[test]
    fn message_start_seeds_the_snapshot() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&message_start()).unwrap();
        let msg = acc.finish().unwrap();
        assert_eq!(msg.id.as_str(), "msg_01");
        assert_eq!(msg.usage.input_tokens, 5);
        assert!(msg.content.is_empty());
    }

    #[test]
    fn finish_without_message_start_is_a_stream_error() {
        let result = MessageAccumulator::new().finish();
        assert!(
            matches!(result, Err(Error::Stream(_))),
            "expected Error::Stream, got {result:?}"
        );
    }

    #[test]
    fn json_bufs_is_seeded_to_match_a_non_empty_message_start_content() {
        // message_start seeds json_bufs to the length of the pre-existing
        // content it carries (spec: json_bufs stays index-parallel to
        // snapshot.content). Starting a tool block *after* that pre-seeded
        // block, at index 1, routes its buffer through a non-zero starting
        // offset: if the seed were `Vec::new()` instead, the buffer pushed
        // for this block would land at json_bufs[0] rather than
        // json_bufs[1], and the input below would never assemble.
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&event(
            r#"{"type":"message_start","message":{
                "id":"msg_01","type":"message","role":"assistant",
                "model":"claude-sonnet-4-5",
                "content":[{"type":"text","text":"seed"}],
                "stop_reason":null,"stop_sequence":null,
                "usage":{"input_tokens":5,"output_tokens":0}}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(&format!(
            r#"{{"type":"content_block_start","index":1,"content_block":{TOOL_BLOCK}}}"#
        )))
        .unwrap();
        acc.accumulate(&json_delta(1, r#"{"city":"#)).unwrap();
        acc.accumulate(&json_delta(1, r#""Paris"}"#)).unwrap();
        acc.accumulate(&block_stop(1)).unwrap();

        let msg = acc.finish().unwrap();
        assert_eq!(msg.content.len(), 2);
        match &msg.content[1] {
            ContentBlock::ToolUse(tu) => {
                assert_eq!(tu.input, serde_json::json!({"city": "Paris"}));
            }
            other => panic!("expected tool-use block, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_message_start_replaces_the_snapshot_and_resets_json_bufs() {
        // Parks an incomplete buffer at json_bufs[0] for the first
        // message's tool block — deliberately never flushed by a
        // content_block_stop, so it is still live when the second
        // message_start arrives.
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&message_start()).unwrap();
        acc.accumulate(&event(&format!(
            r#"{{"type":"content_block_start","index":0,"content_block":{TOOL_BLOCK}}}"#
        )))
        .unwrap();
        acc.accumulate(&json_delta(0, r#"{"city":"#)).unwrap();

        acc.accumulate(&event(
            r#"{"type":"message_start","message":{
                "id":"msg_02","type":"message","role":"assistant",
                "model":"claude-sonnet-4-5","content":[],
                "stop_reason":null,"stop_sequence":null,
                "usage":{"input_tokens":9,"output_tokens":0}}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(&format!(
            r#"{{"type":"content_block_start","index":0,"content_block":{TOOL_BLOCK_2}}}"#
        )))
        .unwrap();
        // If json_bufs had not been reset, this fragment would land after
        // the first message's leftover `{"city":`, producing malformed
        // JSON and an Error::Serde here instead of a clean parse.
        acc.accumulate(&json_delta(0, r#"{"zone":"UTC"}"#)).unwrap();
        acc.accumulate(&block_stop(0)).unwrap();

        let msg = acc.finish().unwrap();
        assert_eq!(msg.id.as_str(), "msg_02");
        assert_eq!(msg.usage.input_tokens, 9);
        assert_eq!(
            msg.content.len(),
            1,
            "the first message's blocks must not survive a second message_start"
        );
        match &msg.content[0] {
            ContentBlock::ToolUse(tu) => {
                assert_eq!(tu.id.as_str(), "toolu_02");
                assert_eq!(tu.input, serde_json::json!({"zone": "UTC"}));
            }
            other => panic!("expected the second message's tool-use block, got {other:?}"),
        }
    }

    // ── content_block_start ────────────────────────────────────────────────

    #[test]
    fn content_block_start_appends_at_the_next_position() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&message_start()).unwrap();
        acc.accumulate(&event(
            r#"{"type":"content_block_start","index":0,
                "content_block":{"type":"text","text":""}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(
            r#"{"type":"content_block_start","index":1,
                "content_block":{"type":"text","text":"seed"}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        assert_eq!(msg.content.len(), 2);
        match (&msg.content[0], &msg.content[1]) {
            (ContentBlock::Text(a), ContentBlock::Text(b)) => {
                assert_eq!(a.text, "");
                assert_eq!(b.text, "seed");
            }
            other => panic!("unexpected blocks: {other:?}"),
        }
    }

    #[test]
    fn content_block_start_with_a_gap_is_a_stream_error() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&message_start()).unwrap();
        let result = acc.accumulate(&event(
            r#"{"type":"content_block_start","index":3,
                "content_block":{"type":"text","text":""}}"#,
        ));
        assert!(
            matches!(result, Err(Error::Stream(_))),
            "expected Error::Stream for a gapped index, got {result:?}"
        );
    }

    #[test]
    fn gapped_content_block_start_does_not_fabricate_placeholder_blocks() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&message_start()).unwrap();
        let _ = acc.accumulate(&event(
            r#"{"type":"content_block_start","index":3,
                "content_block":{"type":"text","text":""}}"#,
        ));
        let msg = acc.finish().unwrap();
        assert!(
            msg.content.is_empty(),
            "a rejected start must leave no blocks behind, got {:?}",
            msg.content
        );
    }

    #[test]
    fn content_block_start_before_message_start_is_ignored() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&event(
            r#"{"type":"content_block_start","index":0,
                "content_block":{"type":"text","text":""}}"#,
        ))
        .unwrap();
        assert!(matches!(acc.finish(), Err(Error::Stream(_))));
    }

    // ── content_block_delta ────────────────────────────────────────────────

    fn started(blocks: &[&str]) -> MessageAccumulator {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&message_start()).unwrap();
        for (i, block) in blocks.iter().enumerate() {
            acc.accumulate(&event(&format!(
                r#"{{"type":"content_block_start","index":{i},"content_block":{block}}}"#
            )))
            .unwrap();
        }
        acc
    }

    #[test]
    fn text_deltas_concatenate() {
        let mut acc = started(&[r#"{"type":"text","text":""}"#]);
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"text_delta","text":"foo"}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"text_delta","text":"bar"}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::Text(tb) => assert_eq!(tb.text, "foobar"),
            other => panic!("expected text block, got {other:?}"),
        }
    }

    #[test]
    fn citations_delta_accumulates_onto_text_block() {
        let mut acc = started(&[r#"{"type":"text","text":""}"#]);
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"citations_delta",
                "citation":{"type":"page_location","cited_text":"x","document_index":0,
                "start_page_number":1,"end_page_number":2}}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"citations_delta",
                "citation":{"type":"page_location","cited_text":"y","document_index":0,
                "start_page_number":3,"end_page_number":4}}}"#,
        ))
        .unwrap();
        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::Text(tb) => assert_eq!(tb.citations.as_ref().unwrap().len(), 2),
            other => panic!("wrong block: {other:?}"),
        }
    }

    #[test]
    fn thinking_deltas_concatenate() {
        let mut acc = started(&[r#"{"type":"thinking","thinking":""}"#]);
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"thinking_delta","thinking":"one "}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"thinking_delta","thinking":"two"}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::Thinking(tb) => assert_eq!(tb.thinking, "one two"),
            other => panic!("expected thinking block, got {other:?}"),
        }
    }

    #[test]
    fn signature_delta_replaces_rather_than_concatenating() {
        let mut acc = started(&[r#"{"type":"thinking","thinking":""}"#]);
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"signature_delta","signature":"first"}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"signature_delta","signature":"second"}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::Thinking(tb) => assert_eq!(tb.signature.as_deref(), Some("second")),
            other => panic!("expected thinking block, got {other:?}"),
        }
    }

    #[test]
    fn delta_with_out_of_range_index_is_a_no_op() {
        let mut acc = started(&[r#"{"type":"text","text":"kept"}"#]);
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":9,
                "delta":{"type":"text_delta","text":"dropped"}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            ContentBlock::Text(tb) => assert_eq!(tb.text, "kept"),
            other => panic!("expected text block, got {other:?}"),
        }
    }

    #[test]
    fn delta_kind_not_matching_the_block_kind_is_a_no_op() {
        let mut acc = started(&[r#"{"type":"text","text":"kept"}"#]);
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"thinking_delta","thinking":"dropped"}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::Text(tb) => assert_eq!(tb.text, "kept"),
            other => panic!("expected text block, got {other:?}"),
        }
    }

    #[test]
    fn unknown_delta_kind_is_a_no_op() {
        let mut acc = started(&[r#"{"type":"text","text":"kept"}"#]);
        acc.accumulate(&event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"citations_delta","citation":{"cited_text":"x"}}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::Text(tb) => assert_eq!(tb.text, "kept"),
            other => panic!("expected text block, got {other:?}"),
        }
    }

    // ── input_json_delta + content_block_stop ──────────────────────────────

    const TOOL_BLOCK: &str =
        r#"{"type":"tool_use","id":"toolu_01","name":"get_weather","input":{}}"#;
    const TOOL_BLOCK_2: &str =
        r#"{"type":"tool_use","id":"toolu_02","name":"get_time","input":{}}"#;
    const SERVER_TOOL_USE_BLOCK: &str =
        r#"{"type":"server_tool_use","id":"srvtoolu_01","name":"web_search","input":{}}"#;

    fn json_delta(index: usize, fragment: &str) -> StreamEvent {
        event(&format!(
            r#"{{"type":"content_block_delta","index":{index},
                "delta":{{"type":"input_json_delta","partial_json":{}}}}}"#,
            serde_json::to_string(fragment).unwrap()
        ))
    }

    fn block_stop(index: usize) -> StreamEvent {
        event(&format!(
            r#"{{"type":"content_block_stop","index":{index}}}"#
        ))
    }

    #[test]
    fn tool_input_json_buffers_across_deltas_and_parses_at_stop() {
        let mut acc = started(&[TOOL_BLOCK]);
        acc.accumulate(&json_delta(0, r#"{"city":"#)).unwrap();
        acc.accumulate(&json_delta(0, r#""Paris"}"#)).unwrap();
        acc.accumulate(&block_stop(0)).unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::ToolUse(tu) => {
                assert_eq!(tu.input, serde_json::json!({"city": "Paris"}));
            }
            other => panic!("expected tool-use block, got {other:?}"),
        }
    }

    #[test]
    fn server_tool_use_input_json_buffers_across_deltas_and_parses_at_stop() {
        let mut acc = started(&[SERVER_TOOL_USE_BLOCK]);
        acc.accumulate(&json_delta(0, r#"{"query":"#)).unwrap();
        acc.accumulate(&json_delta(0, r#""rust"}"#)).unwrap();
        acc.accumulate(&block_stop(0)).unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::ServerToolUse(su) => {
                assert_eq!(su.input, serde_json::json!({"query": "rust"}));
            }
            other => panic!("expected server-tool-use block, got {other:?}"),
        }
    }

    #[test]
    fn empty_partial_json_leaves_the_input_untouched_and_raises_no_error() {
        let mut acc = started(&[TOOL_BLOCK]);
        acc.accumulate(&json_delta(0, "")).unwrap();
        acc.accumulate(&block_stop(0)).unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::ToolUse(tu) => assert_eq!(tu.input, serde_json::json!({})),
            other => panic!("expected tool-use block, got {other:?}"),
        }
    }

    #[test]
    fn tool_block_never_stopped_keeps_the_start_events_input_on_finish() {
        // `{"a": 1,` is the case where Python's eager partial parser
        // (jiter, partial_mode=True) would salvage the complete leading
        // pair and hand the caller `{"a": 1}` even though the block never
        // closed. This accumulator never parses a buffer that has no
        // matching content_block_stop, so `finish()` must return the
        // tool block's start-event input (`{}`) untouched, not a partial
        // parse and not an error.
        let mut acc = started(&[TOOL_BLOCK]);
        acc.accumulate(&json_delta(0, r#"{"a": 1,"#)).unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::ToolUse(tu) => assert_eq!(tu.input, serde_json::json!({})),
            other => panic!("expected tool-use block, got {other:?}"),
        }
    }

    #[test]
    fn malformed_accumulated_tool_json_is_a_serde_error_at_stop() {
        let mut acc = started(&[TOOL_BLOCK]);
        acc.accumulate(&json_delta(0, r#"{"city":"#)).unwrap();
        let result = acc.accumulate(&block_stop(0));
        match result {
            Err(Error::Serde { context, .. }) => assert_eq!(context, "ToolUseBlock.input"),
            other => panic!("expected Error::Serde, got {other:?}"),
        }
    }

    #[test]
    fn two_tool_blocks_buffer_independently() {
        let mut acc = started(&[TOOL_BLOCK, TOOL_BLOCK_2]);
        acc.accumulate(&json_delta(0, r#"{"city":"#)).unwrap();
        acc.accumulate(&json_delta(1, r#"{"zone":"#)).unwrap();
        acc.accumulate(&json_delta(0, r#""Paris"}"#)).unwrap();
        acc.accumulate(&json_delta(1, r#""UTC"}"#)).unwrap();
        acc.accumulate(&block_stop(0)).unwrap();
        acc.accumulate(&block_stop(1)).unwrap();

        let msg = acc.finish().unwrap();
        match (&msg.content[0], &msg.content[1]) {
            (ContentBlock::ToolUse(a), ContentBlock::ToolUse(b)) => {
                assert_eq!(a.input, serde_json::json!({"city": "Paris"}));
                assert_eq!(b.input, serde_json::json!({"zone": "UTC"}));
            }
            other => panic!("expected two tool-use blocks, got {other:?}"),
        }
    }

    #[test]
    fn input_json_delta_addressed_at_a_text_block_is_a_no_op() {
        // The fragment is deliberately malformed: if the guard that keeps
        // input_json_delta out of a non-tool block's buffer were removed,
        // this fragment would still land in json_bufs[0] and fail to parse
        // at content_block_stop, turning the no-op into a spurious
        // Error::Serde. A well-formed fragment would parse either way and
        // would not catch that regression.
        let mut acc = started(&[r#"{"type":"text","text":"kept"}"#]);
        acc.accumulate(&json_delta(0, r#"{"a":"#)).unwrap();
        acc.accumulate(&block_stop(0)).unwrap();

        let msg = acc.finish().unwrap();
        match &msg.content[0] {
            ContentBlock::Text(tb) => assert_eq!(tb.text, "kept"),
            other => panic!("expected text block, got {other:?}"),
        }
    }

    #[test]
    fn content_block_stop_with_out_of_range_index_is_a_no_op() {
        let mut acc = started(&[r#"{"type":"text","text":"kept"}"#]);
        acc.accumulate(&block_stop(9)).unwrap();

        let msg = acc.finish().unwrap();
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            ContentBlock::Text(tb) => assert_eq!(tb.text, "kept"),
            other => panic!("expected text block, got {other:?}"),
        }
    }

    #[test]
    fn content_block_stop_for_a_text_block_leaves_its_content_unchanged() {
        // A stop on a text block alone can't discriminate whether the
        // ContentBlockStop arm actually ran finish_block, since finish_block
        // is a no-op for a block with no buffered JSON either way. Pairing it
        // with a tool block whose input only assembles if finish_block is
        // reached gives the assertion a target: removing the arm entirely
        // leaves the tool input at its unparsed start-event value.
        let mut acc = started(&[r#"{"type":"text","text":"hi"}"#, TOOL_BLOCK]);
        acc.accumulate(&json_delta(1, r#"{"city":"Paris"}"#))
            .unwrap();
        acc.accumulate(&block_stop(0)).unwrap();
        acc.accumulate(&block_stop(1)).unwrap();

        let msg = acc.finish().unwrap();
        match (&msg.content[0], &msg.content[1]) {
            (ContentBlock::Text(tb), ContentBlock::ToolUse(tu)) => {
                assert_eq!(tb.text, "hi");
                assert_eq!(tu.input, serde_json::json!({"city": "Paris"}));
            }
            other => panic!("expected a text block and a tool-use block, got {other:?}"),
        }
    }

    // ── message_delta and inert events ─────────────────────────────────────

    #[test]
    fn message_delta_sets_stop_reason_stop_sequence_and_output_tokens() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&message_start()).unwrap();
        acc.accumulate(&event(
            r#"{"type":"message_delta",
                "delta":{"stop_reason":"stop_sequence","stop_sequence":"END"},
                "usage":{"output_tokens":42}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        assert_eq!(msg.stop_reason, Some(StopReason::StopSequence));
        assert_eq!(
            msg.stop_sequence.as_ref().map(StopSequence::as_str),
            Some("END")
        );
        assert_eq!(msg.usage.output_tokens, 42);
        assert_eq!(
            msg.usage.input_tokens, 5,
            "input_tokens must survive the delta"
        );
    }

    #[test]
    fn message_delta_without_a_stop_reason_leaves_the_previous_value() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&message_start()).unwrap();
        acc.accumulate(&event(
            r#"{"type":"message_delta",
                "delta":{"stop_reason":"end_turn","stop_sequence":"END"},
                "usage":{"output_tokens":3}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(
            r#"{"type":"message_delta",
                "delta":{"stop_reason":null,"stop_sequence":null},
                "usage":{"output_tokens":7}}"#,
        ))
        .unwrap();

        let msg = acc.finish().unwrap();
        assert_eq!(msg.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(
            msg.stop_sequence.as_ref().map(StopSequence::as_str),
            Some("END"),
            "a later delta's null stop_sequence must not clobber the resolved value"
        );
        assert_eq!(msg.usage.output_tokens, 7);
    }

    #[test]
    fn message_delta_merges_input_side_usage_and_stop_details() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&event(
            r#"{"type":"message_start","message":{
                "id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5",
                "content":[],"stop_reason":null,"stop_sequence":null,
                "usage":{"input_tokens":5,"output_tokens":0}}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(
            r#"{"type":"message_delta",
                "delta":{"stop_reason":"refusal","stop_sequence":null,
                         "stop_details":{"type":"refusal","category":"bio","explanation":null}},
                "usage":{"input_tokens":9,"cache_read_input_tokens":4,"output_tokens":42,
                         "server_tool_use":{"web_search_requests":2,"web_fetch_requests":0}}}"#,
        ))
        .unwrap();
        let msg = acc.finish().unwrap();
        assert_eq!(msg.usage.input_tokens, 9, "overwritten from 5");
        assert_eq!(msg.usage.cache_read_input_tokens, Some(4));
        assert_eq!(msg.usage.output_tokens, 42);
        assert_eq!(
            msg.usage.server_tool_use.map(|s| s.web_search_requests),
            Some(2)
        );
        assert_eq!(
            msg.stop_details.map(|d| d.category),
            Some(Some(RefusalCategory::Bio))
        );
    }

    #[test]
    fn message_delta_leaves_input_tokens_untouched_when_absent() {
        let mut acc = MessageAccumulator::new();
        acc.accumulate(&event(
            r#"{"type":"message_start","message":{
                "id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5",
                "content":[],"stop_reason":null,"stop_sequence":null,
                "usage":{"input_tokens":5,"output_tokens":0}}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(
            r#"{"type":"message_delta","delta":{"stop_reason":null,"stop_sequence":null},
                "usage":{"output_tokens":7}}"#,
        ))
        .unwrap();
        let msg = acc.finish().unwrap();
        assert_eq!(msg.usage.input_tokens, 5, "preserved from message_start");
        assert_eq!(msg.usage.output_tokens, 7);
    }

    #[test]
    fn message_stop_ping_error_and_unknown_events_are_inert() {
        let mut acc = started(&[r#"{"type":"text","text":"kept"}"#]);
        acc.accumulate(&event(r#"{"type":"message_stop"}"#))
            .unwrap();
        acc.accumulate(&event(r#"{"type":"ping"}"#)).unwrap();
        acc.accumulate(&event(
            r#"{"type":"error","error":{"type":"overloaded_error","message":"busy"}}"#,
        ))
        .unwrap();
        acc.accumulate(&event(r#"{"type":"some_future_event","payload":1}"#))
            .unwrap();

        let msg = acc.finish().unwrap();
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            ContentBlock::Text(tb) => assert_eq!(tb.text, "kept"),
            other => panic!("expected text block, got {other:?}"),
        }
        assert_eq!(msg.stop_reason, None);
        assert_eq!(msg.stop_sequence, None);
        assert_eq!(msg.usage.output_tokens, 0);
    }
}
