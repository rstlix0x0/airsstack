//! Opaque tool-call identifier newtype.
//!
//! Exists as its own file to enforce the non-empty invariant for server-issued
//! tool-call identifiers at construction time.
//!
//! Responsibilities:
//! - [`ToolCallId`] — a validated, serializable opaque identifier issued by the
//!   server for each tool call (e.g. `call_abc123`).
//! - [`InvalidToolCallId`] — the error returned when construction fails.
//!
//! Not responsible for tool-call structure — see `chat/tool_call.rs`.

use std::fmt;
use std::str::FromStr;

/// An opaque tool-call identifier issued by the server.
///
/// These are returned in the response's `tool_calls[].id` field and echoed
/// back in the subsequent tool-result message's `tool_call_id` field.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::ToolCallId;
///
/// let id = ToolCallId::new("call_abc123").expect("non-empty id");
/// assert_eq!(id.as_str(), "call_abc123");
///
/// assert!(ToolCallId::new("").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ToolCallId(String);

/// Reasons [`ToolCallId::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidToolCallId {
    /// Input was empty.
    #[error("tool call id must not be empty")]
    Empty,
}

impl ToolCallId {
    /// Construct a `ToolCallId` from a non-empty string.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidToolCallId::Empty`] if `s` is empty.
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidToolCallId> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidToolCallId::Empty);
        }
        Ok(Self(s))
    }

    /// Borrow the validated identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolCallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ToolCallId {
    type Err = InvalidToolCallId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn accepts_nonempty_string() {
        let id = ToolCallId::new("call_abc123").unwrap();
        assert_eq!(id.as_str(), "call_abc123");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(ToolCallId::new("").unwrap_err(), InvalidToolCallId::Empty);
    }

    #[test]
    fn from_str_delegates_to_new() {
        assert!("call_xyz".parse::<ToolCallId>().is_ok());
        assert!("".parse::<ToolCallId>().is_err());
    }

    #[test]
    fn display_matches_as_str() {
        let id = ToolCallId::new("call_abc").unwrap();
        assert_eq!(format!("{id}"), id.as_str());
    }

    #[test]
    fn serde_round_trip_transparent() {
        let id = ToolCallId::new("call_abc123").unwrap();
        let j = serde_json::to_string(&id).unwrap();
        assert_eq!(j, "\"call_abc123\"");
        let back: ToolCallId = serde_json::from_str(&j).unwrap();
        assert_eq!(back, id);
    }
}
