//! Decoded SSE chunk for a streaming chat completion.
//!
//! Each `data:` line in a streaming `POST /chat/completions` response decodes
//! to one [`StreamChunk`]. Exists separately from the non-streaming response
//! types because a chunk carries an incremental `delta` (a *partial* message)
//! rather than a complete message, and may carry a mid-stream `error`.
//!
//! Responsibilities:
//! - [`StreamChunk`] — one streamed chunk envelope.
//! - [`ChunkChoice`] — one choice within a chunk.
//! - [`ChunkDelta`] — the incremental message fragment (`role` on the first
//!   chunk, `content` as tokens arrive).
//!
//! Not responsible for:
//! - Driving the stream or enforcing the terminal-on-error rule — that lives in
//!   `stream.rs`. The mid-stream `error` field here is internal signal the
//!   driver converts into [`crate::error::Error::Stream`]; a chunk handed to a
//!   caller never carries it.

use serde::Deserialize;

use crate::chat::message::Role;
use crate::chat::response::FinishReason;
use crate::chat::usage::Usage;

/// One chunk of a streamed chat completion.
///
/// `usage` is populated only on the final chunk. Unknown fields (e.g. the
/// gateway `provider` string) are ignored.
///
/// # Examples
/// ```
/// # #[cfg(feature = "streaming")]
/// # {
/// use openrouter_rs::chat::StreamChunk;
/// let json = r#"{
///     "id": "gen-1", "object": "chat.completion.chunk", "created": 1,
///     "model": "openai/gpt-4o",
///     "choices": [{ "index": 0, "delta": { "content": "Hi" }, "finish_reason": null }]
/// }"#;
/// let chunk: StreamChunk = serde_json::from_str(json).unwrap();
/// assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hi"));
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct StreamChunk {
    /// Generation id (`gen-…`); stable across the chunks of one response.
    pub id: String,
    /// Object discriminator; `"chat.completion.chunk"` for streamed responses.
    pub object: String,
    /// Unix creation timestamp (seconds).
    pub created: u64,
    /// The model producing the completion, as echoed by the server.
    pub model: String,
    /// The streamed choices; one per `index`.
    pub choices: Vec<ChunkChoice>,
    /// Token usage, present only on the final chunk.
    #[serde(default)]
    pub usage: Option<Usage>,
    /// Mid-stream error signal. Internal: the stream driver converts a chunk
    /// carrying this into a terminal [`crate::error::Error::Stream`], so a
    /// chunk delivered to a caller always has this `None`.
    #[serde(default)]
    pub(crate) error: Option<ChunkError>,
}

/// One choice within a [`StreamChunk`].
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ChunkChoice {
    /// Position of this choice in the response.
    pub index: u32,
    /// The incremental message fragment for this choice.
    pub delta: ChunkDelta,
    /// Why generation stopped; `None` until the final chunk for this choice.
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
}

/// The incremental message fragment carried by a [`ChunkChoice`].
///
/// `role` typically appears only on the first chunk; `content` carries the
/// token fragment to append. Both are absent on chunks that only update
/// `finish_reason`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct ChunkDelta {
    /// Author role, present on the first chunk of a choice.
    #[serde(default)]
    pub role: Option<Role>,
    /// The text fragment to append, when present.
    #[serde(default)]
    pub content: Option<String>,
}

/// Mid-stream error object. Only `message` is modeled; the server's `code`
/// field has an inconsistent wire type across providers and is not surfaced.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct ChunkError {
    /// Human-readable description of the mid-stream failure.
    pub message: String,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn decodes_token_chunk() {
        let json = r#"{
            "id": "gen-1", "object": "chat.completion.chunk", "created": 1,
            "model": "openai/gpt-4o",
            "choices": [{ "index": 0, "delta": { "content": "Hel" }, "finish_reason": null }]
        }"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.object, "chat.completion.chunk");
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].index, 0);
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hel"));
        assert!(chunk.choices[0].delta.role.is_none());
        assert!(chunk.choices[0].finish_reason.is_none());
        assert!(chunk.usage.is_none());
        assert!(chunk.error.is_none());
    }

    #[test]
    fn decodes_first_chunk_with_role() {
        let json = r#"{
            "id": "g", "object": "chat.completion.chunk", "created": 1, "model": "x/y",
            "choices": [{ "index": 0, "delta": { "role": "assistant", "content": "" }, "finish_reason": null }]
        }"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].delta.role, Some(Role::Assistant));
    }

    #[test]
    fn decodes_final_chunk_with_finish_reason_and_usage() {
        let json = r#"{
            "id": "g", "object": "chat.completion.chunk", "created": 1, "model": "x/y",
            "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }],
            "usage": { "prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3 }
        }"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Stop));
        assert!(chunk.choices[0].delta.content.is_none());
        assert_eq!(chunk.usage.unwrap().total_tokens, 3);
    }

    #[test]
    fn decodes_error_chunk() {
        let json = r#"{
            "id": "g", "object": "chat.completion.chunk", "created": 1, "model": "x/y",
            "error": { "code": "server_error", "message": "provider disconnected" },
            "choices": [{ "index": 0, "delta": { "content": "" }, "finish_reason": "error" }]
        }"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.error.unwrap().message, "provider disconnected");
        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Error));
    }

    #[test]
    fn ignores_unknown_provider_field() {
        let json = r#"{
            "id": "g", "object": "chat.completion.chunk", "created": 1, "model": "x/y",
            "provider": "openai",
            "choices": [{ "index": 0, "delta": { "content": "ok" } }]
        }"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("ok"));
        assert!(chunk.choices[0].finish_reason.is_none());
    }
}
