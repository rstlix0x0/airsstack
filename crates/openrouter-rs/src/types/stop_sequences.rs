//! Stop-sequence list newtype with a max-4 cap.
//!
//! Exists as a distinct type so the OpenRouter wire contract (array of strings,
//! 1–4 entries) is enforced at construction rather than silently truncated or
//! rejected at the server.
//!
//! Responsibilities:
//! - [`StopSequences`] — a validated, non-empty sequence list capped at four entries.
//! - [`InvalidStopSequences`] — the two-variant error returned when construction fails.

/// Reason [`StopSequences::new`] can reject input.
///
/// OpenRouter accepts at most 4 stop sequences; an empty list has no effect and
/// is rejected early to prevent silent no-ops.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum InvalidStopSequences {
    /// The caller supplied an empty list.
    #[error("stop sequences must not be empty")]
    Empty,
    /// The caller supplied more than 4 entries.
    #[error("stop sequences must not exceed 4 (got {0})")]
    TooMany(usize),
}

/// A validated, non-empty list of stop sequences, capped at 4 entries.
///
/// OpenRouter's chat-completions endpoint accepts a `stop` field that is either
/// a string or an array of strings with at most 4 elements. This type models
/// the array form and enforces the cap at construction.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::StopSequences;
///
/// let stop = StopSequences::new(vec!["\n\n".to_owned()]).unwrap();
/// assert_eq!(stop.get(), &["\n\n"]);
/// ```
///
/// Construction fails when the list is empty or exceeds 4 entries:
///
/// ```
/// use openrouter_rs::types::{StopSequences, InvalidStopSequences};
///
/// assert_eq!(
///     StopSequences::new(vec![]).unwrap_err(),
///     InvalidStopSequences::Empty,
/// );
/// assert_eq!(
///     StopSequences::new(vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()])
///         .unwrap_err(),
///     InvalidStopSequences::TooMany(5),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct StopSequences(Vec<String>);

impl StopSequences {
    /// Validate and wrap a list of stop sequences.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidStopSequences::Empty`] when `seqs` is empty.
    /// Returns [`InvalidStopSequences::TooMany`] when `seqs` has more than 4 entries.
    pub fn new(seqs: Vec<String>) -> Result<Self, InvalidStopSequences> {
        if seqs.is_empty() {
            return Err(InvalidStopSequences::Empty);
        }
        if seqs.len() > 4 {
            return Err(InvalidStopSequences::TooMany(seqs.len()));
        }
        Ok(Self(seqs))
    }

    /// The stop sequences as a slice, for wire-format use.
    #[must_use]
    pub fn get(&self) -> &[String] {
        &self.0
    }

    /// Consume the newtype and return the inner `Vec<String>`.
    #[must_use]
    pub fn into_inner(self) -> Vec<String> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use serde_json::json;

    #[test]
    fn empty_list_is_rejected() {
        assert_eq!(
            StopSequences::new(vec![]).unwrap_err(),
            InvalidStopSequences::Empty,
        );
    }

    #[test]
    fn single_entry_is_accepted() {
        let s = StopSequences::new(vec!["\n\n".to_owned()]).unwrap();
        assert_eq!(s.get(), &["\n\n"]);
    }

    #[test]
    fn exactly_four_is_accepted() {
        let seqs: Vec<String> = ["a", "b", "c", "d"].iter().map(|&s| s.to_owned()).collect();
        let s = StopSequences::new(seqs.clone()).unwrap();
        assert_eq!(s.get(), seqs.as_slice());
    }

    #[test]
    fn five_entries_is_rejected_with_count() {
        let seqs: Vec<String> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|&s| s.to_owned())
            .collect();
        assert_eq!(
            StopSequences::new(seqs).unwrap_err(),
            InvalidStopSequences::TooMany(5),
        );
    }

    #[test]
    fn get_returns_slice_of_inner_strings() {
        let s = StopSequences::new(vec!["stop1".to_owned(), "stop2".to_owned()]).unwrap();
        assert_eq!(s.get(), &["stop1", "stop2"]);
    }

    #[test]
    fn into_inner_returns_owned_vec() {
        let seqs = vec!["x".to_owned(), "y".to_owned()];
        let s = StopSequences::new(seqs.clone()).unwrap();
        assert_eq!(s.into_inner(), seqs);
    }

    #[test]
    fn serializes_as_bare_json_array() {
        let s = StopSequences::new(vec!["\n\n".to_owned()]).unwrap();
        assert_eq!(serde_json::to_value(&s).unwrap(), json!(["\n\n"]));
    }

    #[test]
    fn deserializes_from_json_array() {
        let s: StopSequences = serde_json::from_value(json!(["stop"])).unwrap();
        assert_eq!(s.get(), &["stop"]);
    }
}
