//! System-prompt request types for the Messages API.
//!
//! Two wire-format shapes are accepted:
//!
//! - [`SystemPrompt::Text`] — a single string, serialized as a bare JSON
//!   string (`"You are terse."`). The common case.
//! - [`SystemPrompt::Segments`] — an array of typed [`SystemSegment`]
//!   entries. Each segment carries a kind tag and the segment text, and
//!   may grow additional fields (such as per-segment cache breakpoints)
//!   non-breakingly because the struct is `#[non_exhaustive]`.

/// Top-level system-prompt value sent in a `MessageRequest`.
///
/// # Choosing a variant
///
/// - Use [`SystemPrompt::Text`] for the common single-string case. The
///   Anthropic API accepts a bare JSON string here, so this form keeps
///   the wire payload smallest.
/// - Use [`SystemPrompt::Segments`] when the system prompt is composed
///   of multiple logical chunks that benefit from being addressable
///   independently — typically because some chunks are stable across
///   many requests (good cache candidates) while others vary per-call,
///   or because a tool-using agent needs separate instruction blocks
///   for persona / tools / output format.
///
/// # Wire format
///
/// The enum is `#[serde(untagged)]`, so the discriminant is recovered
/// purely from the JSON shape:
///
/// - `Text("...")`  serializes as `"..."` (bare string).
/// - `Segments([s1, s2])` serializes as `[{"type":"text","text":"..."}, ...]`.
///
/// # Examples
///
/// Single-string form:
///
/// ```
/// use clauders::types::SystemPrompt;
/// let p = SystemPrompt::text("You are terse.");
/// assert_eq!(serde_json::to_string(&p).unwrap(), "\"You are terse.\"");
/// ```
///
/// Segmented form, two chunks:
///
/// ```
/// use clauders::types::{SystemPrompt, SystemSegment};
/// let p = SystemPrompt::segments(vec![
///     SystemSegment::text("You are a code reviewer."),
///     SystemSegment::text("Reply in bullet points."),
/// ]);
/// assert_eq!(
///     serde_json::to_string(&p).unwrap(),
///     r#"[{"type":"text","text":"You are a code reviewer."},{"type":"text","text":"Reply in bullet points."}]"#,
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Single bare-string system prompt — serialized as a JSON string.
    Text(String),
    /// Array-of-segments system prompt — serialized as a JSON array.
    Segments(Vec<SystemSegment>),
}

impl SystemPrompt {
    /// Construct a single-string system prompt.
    #[must_use]
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    /// Construct a segmented system prompt from an existing vector.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec is not const-constructible; the body wraps an owned Vec parameter"
    )]
    pub fn segments(v: Vec<SystemSegment>) -> Self {
        Self::Segments(v)
    }
}

/// A single entry in a segmented [`SystemPrompt::Segments`] payload.
///
/// Each segment carries a wire-format type tag ([`SystemSegmentKind`])
/// and the raw text. Use [`SystemSegment::text`] to construct one — the
/// struct is `#[non_exhaustive]`, so external callers cannot use
/// struct-literal syntax (`SystemSegment { kind, text }`) directly.
///
/// `#[non_exhaustive]` is deliberate: the Messages API may attach
/// optional metadata to a segment in the future (per-segment cache
/// breakpoints are the canonical example), and `non_exhaustive` is the
/// standard Rust pattern for "this struct may gain fields in a minor
/// release without it being a breaking change." Until then, the
/// constructor sets every required field and the result serializes to
/// the minimal wire shape.
///
/// # Wire format
///
/// One segment serializes as `{"type":"text","text":"..."}`. The `type`
/// tag is in snake-case form determined by [`SystemSegmentKind`].
///
/// # Examples
///
/// Construct a segment and inspect its fields:
///
/// ```
/// use clauders::types::{SystemSegment, SystemSegmentKind};
/// let s = SystemSegment::text("hello");
/// assert_eq!(s.kind, SystemSegmentKind::Text);
/// assert_eq!(s.text, "hello");
/// ```
///
/// Serialize a single segment to JSON:
///
/// ```
/// use clauders::types::SystemSegment;
/// let s = SystemSegment::text("Use markdown.");
/// assert_eq!(
///     serde_json::to_string(&s).unwrap(),
///     r#"{"type":"text","text":"Use markdown."}"#,
/// );
/// ```
///
/// Combine multiple segments into a full [`SystemPrompt`]:
///
/// ```
/// use clauders::types::{SystemPrompt, SystemSegment};
/// let prompt = SystemPrompt::segments(vec![
///     SystemSegment::text("You are a strict reviewer."),
///     SystemSegment::text("Cite line numbers."),
/// ]);
/// let json = serde_json::to_string(&prompt).unwrap();
/// assert!(json.starts_with("[{\"type\":\"text\","));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct SystemSegment {
    /// Wire-format type tag for the segment.
    #[serde(rename = "type")]
    pub kind: SystemSegmentKind,
    /// Segment text content.
    pub text: String,
}

/// Wire-format type tag for a [`SystemSegment`].
///
/// Currently only `Text` is supported by the Messages API; additional
/// variants would be added non-breakingly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SystemSegmentKind {
    /// A plain-text segment.
    Text,
}

impl SystemSegment {
    /// Construct a plain-text segment.
    #[must_use]
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            kind: SystemSegmentKind::Text,
            text: s.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn text_form_serializes_as_bare_string() {
        let p = SystemPrompt::text("hi");
        let j = serde_json::to_string(&p).unwrap();
        assert_eq!(j, "\"hi\"");
    }

    #[test]
    fn segments_form_serializes_as_array() {
        let p = SystemPrompt::segments(vec![SystemSegment::text("hi")]);
        let j = serde_json::to_string(&p).unwrap();
        assert_eq!(j, r#"[{"type":"text","text":"hi"}]"#);
    }

    #[test]
    fn empty_segments_serializes_as_empty_array() {
        let p = SystemPrompt::segments(vec![]);
        let j = serde_json::to_string(&p).unwrap();
        assert_eq!(j, "[]");
    }

    #[test]
    fn segment_kind_serializes_snake_case() {
        let k = SystemSegmentKind::Text;
        let j = serde_json::to_string(&k).unwrap();
        assert_eq!(j, "\"text\"");
    }
}
