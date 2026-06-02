//! Validated schema-name newtype for structured-output definitions.
//!
//! Exists as its own file to enforce the name-character and length constraints
//! the OpenRouter API imposes on schema names at construction time, so
//! downstream code never re-checks the invariant.
//!
//! Responsibilities:
//! - [`SchemaName`] — a validated, serializable schema name.
//! - [`InvalidSchemaName`] — the error returned when construction fails.
//!
//! Not responsible for the `response_format` structure — see
//! `chat/response_format.rs`.

use std::fmt;
use std::str::FromStr;

/// A validated schema name for structured-output definitions.
///
/// The API restricts names to `[A-Za-z0-9_-]`, between 1 and 64 characters.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::SchemaName;
///
/// let name = SchemaName::new("weather").expect("valid name");
/// assert_eq!(name.as_str(), "weather");
///
/// assert!(SchemaName::new("").is_err());
/// assert!(SchemaName::new("name with space").is_err());
/// assert!(SchemaName::new("a".repeat(65)).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SchemaName(String);

/// Reasons [`SchemaName::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidSchemaName {
    /// Input was empty.
    #[error("schema name must not be empty")]
    Empty,
    /// Input exceeded 64 characters.
    #[error("schema name must be at most 64 characters, got {0}")]
    TooLong(usize),
    /// Input contained a character outside `[A-Za-z0-9_-]`.
    #[error("schema name contains invalid character: only [A-Za-z0-9_-] are allowed")]
    InvalidChar,
}

impl SchemaName {
    /// Construct a `SchemaName`, validating charset and length.
    ///
    /// # Errors
    ///
    /// - [`InvalidSchemaName::Empty`] if `s` is empty.
    /// - [`InvalidSchemaName::TooLong`] if `s` exceeds 64 characters.
    /// - [`InvalidSchemaName::InvalidChar`] if `s` contains any character
    ///   outside `[A-Za-z0-9_-]`.
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidSchemaName> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidSchemaName::Empty);
        }
        if s.len() > 64 {
            return Err(InvalidSchemaName::TooLong(s.len()));
        }
        if s.chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        {
            return Err(InvalidSchemaName::InvalidChar);
        }
        Ok(Self(s))
    }

    /// Borrow the validated name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SchemaName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for SchemaName {
    type Err = InvalidSchemaName;

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
        let n = SchemaName::new("weather").unwrap();
        assert_eq!(n.as_str(), "weather");
    }

    #[test]
    fn accepts_name_with_hyphen() {
        assert!(SchemaName::new("weather-report").is_ok());
    }

    #[test]
    fn accepts_exactly_64_chars() {
        let name = "a".repeat(64);
        assert!(SchemaName::new(name).is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(SchemaName::new("").unwrap_err(), InvalidSchemaName::Empty);
    }

    #[test]
    fn rejects_too_long() {
        let name = "a".repeat(65);
        assert!(matches!(
            SchemaName::new(name).unwrap_err(),
            InvalidSchemaName::TooLong(65)
        ));
    }

    #[test]
    fn rejects_bad_char_space() {
        assert_eq!(
            SchemaName::new("name with space").unwrap_err(),
            InvalidSchemaName::InvalidChar
        );
    }

    #[test]
    fn rejects_bad_char_dot() {
        assert_eq!(
            SchemaName::new("schema.name").unwrap_err(),
            InvalidSchemaName::InvalidChar
        );
    }

    #[test]
    fn from_str_delegates_to_new() {
        assert!("weather".parse::<SchemaName>().is_ok());
        assert!("".parse::<SchemaName>().is_err());
    }

    #[test]
    fn display_matches_as_str() {
        let n = SchemaName::new("my_schema").unwrap();
        assert_eq!(format!("{n}"), n.as_str());
    }

    #[test]
    fn serde_round_trip_transparent() {
        let n = SchemaName::new("get_weather").unwrap();
        let j = serde_json::to_string(&n).unwrap();
        assert_eq!(j, "\"get_weather\"");
        let back: SchemaName = serde_json::from_str(&j).unwrap();
        assert_eq!(back, n);
    }
}
