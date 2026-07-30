//! Identifier of a single message within a session transcript.

/// A message uuid, used to anchor a resumed session to a point in its
/// transcript (the binary's `--resume-session-at=<message id>`).
///
/// The value is a message uuid minted by the binary; the SDK trims
/// surrounding whitespace and rejects an empty value, but does not enforce a
/// uuid format (the binary owns the id scheme).
///
/// # Examples
///
/// ```
/// use clauders::agent::types::MessageId;
/// assert!(MessageId::new("6f2c34e3-6d3d-4fe8-8528-060a29ee8194").is_ok());
/// assert!(MessageId::new("   ").is_err());
/// assert_eq!(MessageId::new("  m1  ").expect("valid").as_str(), "m1");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageId(String);

/// Reason [`MessageId::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("message id must be non-empty")]
#[non_exhaustive]
pub struct InvalidMessageId;

impl MessageId {
    /// Validate and wrap a message uuid, trimming surrounding whitespace.
    ///
    /// # Errors
    /// Returns [`InvalidMessageId`] when the value is empty or all whitespace.
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidMessageId> {
        let trimmed = s.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(InvalidMessageId);
        }
        Ok(Self(trimmed))
    }

    /// Borrow the underlying id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::MessageId;

    #[test]
    fn rejects_empty_and_whitespace() {
        assert!(MessageId::new("").is_err());
        assert!(MessageId::new("   ").is_err());
        assert!(MessageId::new("\t\n").is_err());
    }

    #[test]
    fn accepts_and_trims_a_valid_id() {
        let id = MessageId::new("  msg_1  ").expect("valid");
        assert_eq!(id.as_str(), "msg_1");
    }
}
