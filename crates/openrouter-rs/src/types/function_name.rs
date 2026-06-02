//! Validated function-name newtype for tool definitions.
//!
//! Exists as its own file to enforce the name-character and length constraints
//! the OpenRouter API imposes on tool function names at construction time, so
//! downstream code never re-checks the invariant.
//!
//! Responsibilities:
//! - [`FunctionName`] — a validated, serializable function name.
//! - [`InvalidFunctionName`] — the error returned when construction fails.
//!
//! Not responsible for tool-definition structure — see `chat/tool.rs`.

use std::fmt;
use std::str::FromStr;

/// A validated tool function name accepted by the OpenRouter chat API.
///
/// The API restricts names to `[A-Za-z0-9_-]`, between 1 and 64 characters.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::FunctionName;
///
/// let name = FunctionName::new("search_books").expect("valid name");
/// assert_eq!(name.as_str(), "search_books");
///
/// assert!(FunctionName::new("").is_err());
/// assert!(FunctionName::new("name with space").is_err());
/// assert!(FunctionName::new("a".repeat(65)).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FunctionName(String);

/// Reasons [`FunctionName::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidFunctionName {
    /// Input was empty.
    #[error("function name must not be empty")]
    Empty,
    /// Input exceeded 64 characters.
    #[error("function name must be at most 64 characters, got {0}")]
    TooLong(usize),
    /// Input contained a character outside `[A-Za-z0-9_-]`.
    #[error("function name contains invalid character: only [A-Za-z0-9_-] are allowed")]
    InvalidChar,
}

impl FunctionName {
    /// Construct a `FunctionName`, validating charset and length.
    ///
    /// # Errors
    ///
    /// - [`InvalidFunctionName::Empty`] if `s` is empty.
    /// - [`InvalidFunctionName::TooLong`] if `s` exceeds 64 characters.
    /// - [`InvalidFunctionName::InvalidChar`] if `s` contains any character
    ///   outside `[A-Za-z0-9_-]`.
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidFunctionName> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidFunctionName::Empty);
        }
        if s.len() > 64 {
            return Err(InvalidFunctionName::TooLong(s.len()));
        }
        if s.chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        {
            return Err(InvalidFunctionName::InvalidChar);
        }
        Ok(Self(s))
    }

    /// Borrow the validated name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for FunctionName {
    type Err = InvalidFunctionName;

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
    fn accepts_simple_alphanumeric_name() {
        let n = FunctionName::new("search_books").unwrap();
        assert_eq!(n.as_str(), "search_books");
    }

    #[test]
    fn accepts_name_with_hyphen() {
        assert!(FunctionName::new("get-user-data").is_ok());
    }

    #[test]
    fn accepts_exactly_64_chars() {
        let name = "a".repeat(64);
        assert!(FunctionName::new(name).is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            FunctionName::new("").unwrap_err(),
            InvalidFunctionName::Empty
        );
    }

    #[test]
    fn rejects_too_long() {
        let name = "a".repeat(65);
        assert!(matches!(
            FunctionName::new(name).unwrap_err(),
            InvalidFunctionName::TooLong(65)
        ));
    }

    #[test]
    fn rejects_bad_char_space() {
        assert_eq!(
            FunctionName::new("name with space").unwrap_err(),
            InvalidFunctionName::InvalidChar
        );
    }

    #[test]
    fn rejects_bad_char_dot() {
        assert_eq!(
            FunctionName::new("fn.name").unwrap_err(),
            InvalidFunctionName::InvalidChar
        );
    }

    #[test]
    fn from_str_delegates_to_new() {
        assert!("search_books".parse::<FunctionName>().is_ok());
        assert!("".parse::<FunctionName>().is_err());
    }

    #[test]
    fn display_matches_as_str() {
        let n = FunctionName::new("my_func").unwrap();
        assert_eq!(format!("{n}"), n.as_str());
    }

    #[test]
    fn serde_round_trip_transparent() {
        let n = FunctionName::new("get_weather").unwrap();
        let j = serde_json::to_string(&n).unwrap();
        assert_eq!(j, "\"get_weather\"");
        let back: FunctionName = serde_json::from_str(&j).unwrap();
        assert_eq!(back, n);
    }
}
