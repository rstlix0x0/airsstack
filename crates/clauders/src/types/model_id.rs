//! Claude model identifier newtype.

use std::fmt;

/// A Claude model identifier accepted by the Anthropic Messages API.
///
/// # Choosing a constructor
///
/// - [`ModelId::custom`] is the **primary** entry point. It accepts any
///   non-empty whitespace-free identifier and never goes stale.
/// - The `claude_*` headline-model constructors are a convenience snapshot
///   of the models known to this SDK release. They are typo-proof and
///   IDE-discoverable, but the set is **frozen at SDK build time** —
///   Anthropic ships new models faster than this SDK's release cadence,
///   so for the authoritative current list query the upstream endpoint
///   <https://api.anthropic.com/v1/models> or use the SDK's `models`
///   resource once enabled.
///
/// # Examples
///
/// ```
/// use clauders::types::ModelId;
/// // Headline convenience constructor — current at SDK build time.
/// assert_eq!(ModelId::claude_sonnet_4_5().as_str(), "claude-sonnet-4-5");
///
/// // Primary path — works for any current or future identifier.
/// let custom = ModelId::custom("claude-future-model-1").expect("valid id");
/// assert_eq!(custom.as_str(), "claude-future-model-1");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ModelId(String);

/// Reasons [`ModelId::custom`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidModelId {
    /// Input was empty.
    #[error("model id must not be empty")]
    Empty,
    /// Input contained ASCII or Unicode whitespace.
    #[error("model id must not contain whitespace")]
    Whitespace,
}

impl ModelId {
    /// Construct a `ModelId` for an arbitrary identifier the SDK does not
    /// yet name explicitly.
    ///
    /// # Errors
    /// Returns [`InvalidModelId::Empty`] if `s` is empty, or
    /// [`InvalidModelId::Whitespace`] if `s` contains any whitespace character.
    pub fn custom(s: impl Into<String>) -> Result<Self, InvalidModelId> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidModelId::Empty);
        }
        if s.chars().any(char::is_whitespace) {
            return Err(InvalidModelId::Whitespace);
        }
        Ok(Self(s))
    }

    /// Borrow the validated identifier for wire-format use.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Headline Opus 4.7 model.
    #[must_use]
    pub fn claude_opus_4_7() -> Self {
        Self("claude-opus-4-7".to_owned())
    }

    /// Headline Sonnet 4.6 model.
    #[must_use]
    pub fn claude_sonnet_4_6() -> Self {
        Self("claude-sonnet-4-6".to_owned())
    }

    /// Headline Sonnet 4.5 model.
    #[must_use]
    pub fn claude_sonnet_4_5() -> Self {
        Self("claude-sonnet-4-5".to_owned())
    }

    /// Headline Haiku 4.5 model.
    #[must_use]
    pub fn claude_haiku_4_5() -> Self {
        Self("claude-haiku-4-5".to_owned())
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    fn headline_models_have_expected_wire_strings() {
        assert_eq!(ModelId::claude_opus_4_7().as_str(), "claude-opus-4-7");
        assert_eq!(ModelId::claude_sonnet_4_6().as_str(), "claude-sonnet-4-6");
        assert_eq!(ModelId::claude_sonnet_4_5().as_str(), "claude-sonnet-4-5");
        assert_eq!(ModelId::claude_haiku_4_5().as_str(), "claude-haiku-4-5");
    }

    #[test]
    fn custom_rejects_empty() {
        assert_eq!(ModelId::custom("").unwrap_err(), InvalidModelId::Empty);
    }

    #[test]
    fn custom_rejects_whitespace() {
        assert_eq!(
            ModelId::custom("has space").unwrap_err(),
            InvalidModelId::Whitespace
        );
        assert_eq!(
            ModelId::custom("has\ttab").unwrap_err(),
            InvalidModelId::Whitespace
        );
        assert_eq!(
            ModelId::custom("has\nnewline").unwrap_err(),
            InvalidModelId::Whitespace
        );
    }

    #[test]
    fn round_trips_serde_transparent() {
        let m = ModelId::claude_sonnet_4_5();
        let j = serde_json::to_string(&m).unwrap();
        assert_eq!(j, "\"claude-sonnet-4-5\"");
        let back: ModelId = serde_json::from_str(&j).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn display_matches_as_str() {
        let m = ModelId::claude_opus_4_7();
        assert_eq!(format!("{m}"), m.as_str());
    }
}
