//! `BatchId` newtype — opaque server-generated batch identifier.
//!
//! Exists as a distinct type so call sites cannot accidentally swap a
//! `BatchId` for any other identifier. Non-empty invariant is enforced at
//! construction; downstream code trusts the type as proof.

/// Opaque server-generated identifier for a message batch.
///
/// Obtain by calling [`BatchId::new`] with the raw string returned by the
/// Batches API. The string is preserved verbatim; no further parsing is
/// applied beyond the non-empty check.
///
/// # Examples
///
/// ```
/// use clauders::types::BatchId;
///
/// let id = BatchId::new("msgbatch_01").unwrap();
/// assert_eq!(id.as_str(), "msgbatch_01");
/// assert_eq!(id.to_string(), "msgbatch_01");
///
/// assert!(BatchId::new("").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BatchId(String);

/// Reason [`BatchId::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("BatchId must not be empty")]
pub struct InvalidBatchId;

impl BatchId {
    /// Validate and wrap a string as a batch identifier.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidBatchId`] when `s` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::types::BatchId;
    ///
    /// assert!(BatchId::new("msgbatch_01").is_ok());
    /// assert!(BatchId::new("").is_err());
    /// ```
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidBatchId> {
        let s = s.into();
        if s.is_empty() {
            Err(InvalidBatchId)
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

impl std::fmt::Display for BatchId {
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
        let id = BatchId::new("msgbatch_01").unwrap();
        assert_eq!(id.as_str(), "msgbatch_01");
    }

    #[test]
    fn new_rejects_empty_string() {
        assert_eq!(BatchId::new(""), Err(InvalidBatchId));
    }

    #[test]
    fn display_matches_inner() {
        let id = BatchId::new("msgbatch_xyz").unwrap();
        assert_eq!(id.to_string(), "msgbatch_xyz");
    }

    #[test]
    fn serde_transparent_round_trip() {
        let id = BatchId::new("msgbatch_serde").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""msgbatch_serde""#);
        let back: BatchId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }
}
