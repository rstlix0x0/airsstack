//! Validated provider-slug newtype for routing-preference lists.
//!
//! Exists as its own file to enforce the non-empty, no-whitespace constraint
//! on provider slugs at construction time, so routing-preference code never
//! re-checks the invariant.
//!
//! Responsibilities:
//! - [`ProviderSlug`] — a validated, serializable provider slug.
//! - [`InvalidProviderSlug`] — the error returned when construction fails.
//!
//! Not responsible for the provider-preferences structure — see
//! `chat/provider.rs`.

use std::fmt;
use std::str::FromStr;

/// A validated provider slug used in routing-preference lists.
///
/// Provider slugs identify specific model providers (for example
/// `"openai"`, `"anthropic"`, `"mistralai"`). The only constraints are
/// non-empty and no whitespace — the charset is deliberately open because
/// provider names include hyphens, digits, and lowercase letters.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::ProviderSlug;
///
/// let slug = ProviderSlug::new("openai").expect("valid slug");
/// assert_eq!(slug.as_str(), "openai");
///
/// assert!(ProviderSlug::new("").is_err());
/// assert!(ProviderSlug::new("open ai").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ProviderSlug(String);

/// Reasons [`ProviderSlug::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidProviderSlug {
    /// Input was empty.
    #[error("provider slug must not be empty")]
    Empty,
    /// Input contained whitespace (leading, trailing, or internal).
    #[error("provider slug must not contain whitespace")]
    Whitespace,
}

impl ProviderSlug {
    /// Construct a `ProviderSlug`, validating it is non-empty and has no
    /// whitespace.
    ///
    /// # Errors
    ///
    /// - [`InvalidProviderSlug::Empty`] if `s` is empty.
    /// - [`InvalidProviderSlug::Whitespace`] if `s` contains any whitespace
    ///   character.
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidProviderSlug> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidProviderSlug::Empty);
        }
        if s.chars().any(char::is_whitespace) {
            return Err(InvalidProviderSlug::Whitespace);
        }
        Ok(Self(s))
    }

    /// Borrow the validated slug.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProviderSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ProviderSlug {
    type Err = InvalidProviderSlug;

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
    fn accepts_simple_lowercase_slug() {
        let s = ProviderSlug::new("openai").unwrap();
        assert_eq!(s.as_str(), "openai");
    }

    #[test]
    fn accepts_slug_with_hyphen_and_digits() {
        assert!(ProviderSlug::new("mistral-ai-123").is_ok());
    }

    #[test]
    fn accepts_single_char() {
        assert!(ProviderSlug::new("x").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            ProviderSlug::new("").unwrap_err(),
            InvalidProviderSlug::Empty
        );
    }

    #[test]
    fn rejects_internal_space() {
        assert_eq!(
            ProviderSlug::new("open ai").unwrap_err(),
            InvalidProviderSlug::Whitespace
        );
    }

    #[test]
    fn rejects_leading_whitespace() {
        assert_eq!(
            ProviderSlug::new(" openai").unwrap_err(),
            InvalidProviderSlug::Whitespace
        );
    }

    #[test]
    fn rejects_trailing_whitespace() {
        assert_eq!(
            ProviderSlug::new("openai ").unwrap_err(),
            InvalidProviderSlug::Whitespace
        );
    }

    #[test]
    fn rejects_tab_character() {
        assert_eq!(
            ProviderSlug::new("open\tai").unwrap_err(),
            InvalidProviderSlug::Whitespace
        );
    }

    #[test]
    fn serde_transparent_round_trip() {
        let s = ProviderSlug::new("anthropic").unwrap();
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(j, "\"anthropic\"");
        let back: ProviderSlug = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn from_str_delegates_to_new() {
        assert!("anthropic".parse::<ProviderSlug>().is_ok());
        assert!("".parse::<ProviderSlug>().is_err());
        assert!("a b".parse::<ProviderSlug>().is_err());
    }

    #[test]
    fn display_matches_as_str() {
        let s = ProviderSlug::new("openai").unwrap();
        assert_eq!(format!("{s}"), s.as_str());
    }
}
