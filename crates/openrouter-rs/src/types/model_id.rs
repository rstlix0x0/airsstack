//! OpenRouter model-identifier newtype.

use std::fmt;
use std::str::FromStr;

/// An OpenRouter model identifier, following the `provider/model-name` slug
/// pattern (for example `anthropic/claude-sonnet-4-5`, `openai/gpt-4o`).
///
/// Routing-hint suffixes are accepted verbatim (`openai/gpt-4o:nitro` for the
/// highest-throughput provider, `…:floor` for the lowest price). The
/// authoritative model catalogue is the OpenRouter models endpoint
/// <https://openrouter.ai/api/v1/models>; this SDK does not freeze a list of
/// known models, so [`ModelId::custom`] (or its [`FromStr`] equivalent) is the
/// single entry point and never goes stale.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::ModelId;
/// let model = ModelId::custom("anthropic/claude-sonnet-4-5").expect("valid id");
/// assert_eq!(model.as_str(), "anthropic/claude-sonnet-4-5");
/// assert!(ModelId::custom("has space").is_err());
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
    /// Construct a `ModelId` from a `provider/model` slug.
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
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ModelId {
    type Err = InvalidModelId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::custom(s)
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use std::str::FromStr;

    #[test]
    fn custom_accepts_provider_model_slug() {
        let m = ModelId::custom("anthropic/claude-sonnet-4-5").unwrap();
        assert_eq!(m.as_str(), "anthropic/claude-sonnet-4-5");
    }

    #[test]
    fn custom_accepts_routing_suffix() {
        assert!(ModelId::custom("openai/gpt-4o:nitro").is_ok());
        assert!(ModelId::custom("deepseek/deepseek-r1:floor").is_ok());
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
    }

    #[test]
    fn from_str_delegates_to_custom() {
        assert_eq!(
            ModelId::from_str("openai/gpt-4o").unwrap().as_str(),
            "openai/gpt-4o"
        );
        assert!(ModelId::from_str("bad id").is_err());
    }

    #[test]
    fn round_trips_serde_transparent() {
        let m = ModelId::custom("google/gemini-2.0-flash-001").unwrap();
        let j = serde_json::to_string(&m).unwrap();
        assert_eq!(j, "\"google/gemini-2.0-flash-001\"");
        let back: ModelId = serde_json::from_str(&j).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn display_matches_as_str() {
        let m = ModelId::custom("meta-llama/llama-3.3-70b-instruct").unwrap();
        assert_eq!(format!("{m}"), m.as_str());
    }
}
