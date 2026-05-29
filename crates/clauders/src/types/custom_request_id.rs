//! `CustomRequestId` newtype — caller-supplied batch row correlation identifier.
//!
//! Exists as a distinct type so callers cannot accidentally swap a
//! `CustomRequestId` for a `BatchId` or other opaque identifier. Non-empty
//! invariant is enforced at construction.

/// Caller-supplied identifier that correlates a batch row with its result.
///
/// Supplied by the caller when building a [`crate::messages::BatchRequest`];
/// returned unchanged in each [`crate::messages::BatchResultRow`] so callers
/// can map results back to their original inputs.
///
/// # Examples
///
/// ```
/// use clauders::types::CustomRequestId;
///
/// let id = CustomRequestId::new("my-row-1").unwrap();
/// assert_eq!(id.as_str(), "my-row-1");
/// assert_eq!(id.to_string(), "my-row-1");
///
/// assert!(CustomRequestId::new("").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CustomRequestId(String);

/// Reason [`CustomRequestId::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("CustomRequestId must not be empty")]
pub struct InvalidCustomRequestId;

impl CustomRequestId {
    /// Validate and wrap a string as a batch row correlation identifier.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidCustomRequestId`] when `s` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::types::CustomRequestId;
    ///
    /// assert!(CustomRequestId::new("row-001").is_ok());
    /// assert!(CustomRequestId::new("").is_err());
    /// ```
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidCustomRequestId> {
        let s = s.into();
        if s.is_empty() {
            Err(InvalidCustomRequestId)
        } else {
            Ok(Self(s))
        }
    }

    /// Borrow the validated identifier value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CustomRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
    fn new_accepts_non_empty_string() {
        let id = CustomRequestId::new("row-001").unwrap();
        assert_eq!(id.as_str(), "row-001");
    }

    #[test]
    fn new_rejects_empty_string() {
        assert_eq!(CustomRequestId::new(""), Err(InvalidCustomRequestId));
    }

    #[test]
    fn display_matches_inner() {
        let id = CustomRequestId::new("row-xyz").unwrap();
        assert_eq!(id.to_string(), "row-xyz");
    }

    #[test]
    fn serde_transparent_round_trip() {
        let id = CustomRequestId::new("row-serde").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""row-serde""#);
        let back: CustomRequestId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }
}
