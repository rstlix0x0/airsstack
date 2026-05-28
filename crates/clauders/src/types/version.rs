//! Anthropic API version + beta-header newtypes.

use std::fmt;

/// Value of the required `anthropic-version` request header.
///
/// Use [`AnthropicVersion::V_2023_06_01`] for the current stable version.
/// Custom versions can be constructed via [`AnthropicVersion::custom`] for
/// forward-compat with Anthropic releases this SDK version predates.
///
/// # Examples
///
/// ```
/// use clauders::types::AnthropicVersion;
/// assert_eq!(AnthropicVersion::V_2023_06_01.as_str(), "2023-06-01");
/// assert_eq!(AnthropicVersion::default(), AnthropicVersion::V_2023_06_01);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnthropicVersion(VersionRepr);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum VersionRepr {
    Static(&'static str),
    Owned(String),
}

/// Reasons [`AnthropicVersion::custom`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidAnthropicVersion {
    /// Input was empty.
    #[error("anthropic-version must not be empty")]
    Empty,
    /// Input contained non-printable ASCII or whitespace.
    #[error("anthropic-version must be ASCII without whitespace")]
    BadChars,
}

impl AnthropicVersion {
    /// The stable Anthropic API version.
    pub const V_2023_06_01: Self = Self(VersionRepr::Static("2023-06-01"));

    /// Construct a custom version string for forward-compat with releases
    /// this SDK version predates.
    ///
    /// # Errors
    /// Returns [`InvalidAnthropicVersion::Empty`] if `s` is empty, or
    /// [`InvalidAnthropicVersion::BadChars`] if `s` contains non-printable
    /// ASCII or whitespace.
    pub fn custom(s: impl Into<String>) -> Result<Self, InvalidAnthropicVersion> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidAnthropicVersion::Empty);
        }
        if !s.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(InvalidAnthropicVersion::BadChars);
        }
        Ok(Self(VersionRepr::Owned(s)))
    }

    /// Borrow the version string for HTTP header construction.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            VersionRepr::Static(s) => s,
            VersionRepr::Owned(s) => s.as_str(),
        }
    }
}

impl Default for AnthropicVersion {
    fn default() -> Self {
        Self::V_2023_06_01
    }
}

impl fmt::Display for AnthropicVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single entry in the `anthropic-beta` request header.
///
/// Beta flags are opaque strings such as `prompt-caching-2024-07-31`.
/// Validated to the charset `[a-z0-9._-]+`.
///
/// # Examples
///
/// ```
/// use clauders::types::BetaHeader;
/// let h = BetaHeader::new("prompt-caching-2024-07-31").expect("valid beta");
/// assert_eq!(h.as_str(), "prompt-caching-2024-07-31");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BetaHeader(String);

/// Reasons [`BetaHeader::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidBetaHeader {
    /// Input was empty.
    #[error("beta header value must not be empty")]
    Empty,
    /// Input contained characters outside `[a-z0-9._-]+`.
    #[error("beta header value must match [a-z0-9._-]+")]
    BadChars,
}

impl BetaHeader {
    /// Validate and wrap a beta-flag string.
    ///
    /// # Errors
    /// Returns [`InvalidBetaHeader::Empty`] if `s` is empty, or
    /// [`InvalidBetaHeader::BadChars`] if `s` contains characters outside
    /// the `[a-z0-9._-]+` charset.
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidBetaHeader> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidBetaHeader::Empty);
        }
        if !s.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        }) {
            return Err(InvalidBetaHeader::BadChars);
        }
        Ok(Self(s))
    }

    /// Borrow the validated string for HTTP header construction.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BetaHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn version_default_is_2023_06_01() {
        assert_eq!(AnthropicVersion::default(), AnthropicVersion::V_2023_06_01);
        assert_eq!(AnthropicVersion::default().as_str(), "2023-06-01");
    }

    #[test]
    fn version_custom_validates() {
        assert!(AnthropicVersion::custom("2024-12-01").is_ok());
        assert_eq!(
            AnthropicVersion::custom("").unwrap_err(),
            InvalidAnthropicVersion::Empty
        );
        assert_eq!(
            AnthropicVersion::custom("with space").unwrap_err(),
            InvalidAnthropicVersion::BadChars
        );
    }

    #[test]
    fn version_display_matches_as_str() {
        let v = AnthropicVersion::default();
        assert_eq!(format!("{v}"), v.as_str());
    }

    #[test]
    fn beta_header_validates_charset() {
        assert!(BetaHeader::new("prompt-caching-2024-07-31").is_ok());
        assert_eq!(
            BetaHeader::new("UPPER").unwrap_err(),
            InvalidBetaHeader::BadChars
        );
        assert_eq!(BetaHeader::new("").unwrap_err(), InvalidBetaHeader::Empty);
        assert_eq!(
            BetaHeader::new("with space").unwrap_err(),
            InvalidBetaHeader::BadChars
        );
    }

    #[test]
    fn beta_header_display_matches_as_str() {
        let h = BetaHeader::new("prompt-caching-2024-07-31").unwrap();
        assert_eq!(format!("{h}"), h.as_str());
    }
}
