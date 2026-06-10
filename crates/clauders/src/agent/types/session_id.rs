//! Session identifier minted by the `claude` binary.

/// An opaque session identifier returned by the binary in result frames.
///
/// The value is server-assigned and echoed back verbatim on control
/// requests; the SDK does not validate or interpret its contents.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    /// Wrap a server-minted session id.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the underlying identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::SessionId;

    #[test]
    fn wraps_and_borrows() {
        let id = SessionId::new("sess_abc123");
        assert_eq!(id.as_str(), "sess_abc123");
    }

    #[test]
    fn round_trips_through_json() {
        let id = SessionId::new("sess_x");
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"sess_x\"");
        let back: SessionId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }
}
