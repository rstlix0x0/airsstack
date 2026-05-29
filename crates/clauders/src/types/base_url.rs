//! Validated base-URL newtype for the Anthropic API endpoint.
//!
//! Exists so the SDK never carries a raw `url::Url` on its public surface.
//! A bare URL has no scheme guarantee, and re-exporting the `url` crate's
//! type would turn every `url` version bump into a breaking change for
//! downstream callers. [`BaseUrl`] wraps the URL, rejects any scheme other
//! than `http` / `https` at construction, and exposes only a string view —
//! the inner `url::Url` stays private to this crate.
//!
//! Responsibilities:
//! - Declare [`BaseUrl`] and its validating constructor [`BaseUrl::parse`].
//! - Declare [`InvalidBaseUrl`], the construction-time rejection reason.
//!
//! Not responsible for:
//! - Request-URI assembly — the request layer composes endpoint paths onto
//!   the validated base; this type only guarantees the base is well-formed.

use std::fmt;

/// Base URL the SDK builds every request against.
///
/// Construct via [`BaseUrl::parse`], which accepts only `http` and `https`
/// schemes. `http` is permitted so callers can target a local proxy or a
/// test server (for example `http://127.0.0.1:8080`); schemes such as
/// `file`, `data`, or `ftp` are rejected.
///
/// # Examples
///
/// ```
/// use clauders::types::BaseUrl;
/// let base = BaseUrl::parse("https://api.anthropic.com").expect("valid https URL");
/// assert_eq!(base.as_str(), "https://api.anthropic.com/");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BaseUrl(url::Url);

/// Reasons [`BaseUrl::parse`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidBaseUrl {
    /// Input did not parse as an absolute URL.
    #[error("base URL is not a valid absolute URL: {0}")]
    Malformed(String),
    /// Input parsed but used a scheme other than `http` / `https`.
    #[error("base URL scheme must be http or https, got {0:?}")]
    UnsupportedScheme(String),
}

impl BaseUrl {
    /// Parse and validate a base URL.
    ///
    /// # Errors
    /// Returns [`InvalidBaseUrl::Malformed`] when `s` is not a valid
    /// absolute URL, or [`InvalidBaseUrl::UnsupportedScheme`] when the
    /// scheme is not `http` or `https`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::types::{BaseUrl, InvalidBaseUrl};
    /// assert!(BaseUrl::parse("http://127.0.0.1:8080").is_ok());
    /// let err = BaseUrl::parse("ftp://example.com").unwrap_err();
    /// assert!(matches!(err, InvalidBaseUrl::UnsupportedScheme(_)));
    /// ```
    pub fn parse(s: impl AsRef<str>) -> Result<Self, InvalidBaseUrl> {
        let url =
            url::Url::parse(s.as_ref()).map_err(|e| InvalidBaseUrl::Malformed(e.to_string()))?;
        match url.scheme() {
            "http" | "https" => Ok(Self(url)),
            other => Err(InvalidBaseUrl::UnsupportedScheme(other.to_owned())),
        }
    }

    /// Borrow the URL as a string slice for request construction.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for BaseUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
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
    fn parse_accepts_https() {
        let base = BaseUrl::parse("https://api.anthropic.com").unwrap();
        assert_eq!(base.as_str(), "https://api.anthropic.com/");
    }

    #[test]
    fn parse_accepts_http_loopback() {
        // `http` stays valid so callers can target local proxies and test
        // servers; the policy only blocks non-web schemes.
        let base = BaseUrl::parse("http://127.0.0.1:8080").unwrap();
        assert_eq!(base.as_str(), "http://127.0.0.1:8080/");
    }

    #[test]
    fn parse_rejects_non_web_schemes() {
        for bad in [
            "file:///etc/passwd",
            "ftp://example.com",
            "data:text/plain,hi",
        ] {
            assert!(
                matches!(
                    BaseUrl::parse(bad).unwrap_err(),
                    InvalidBaseUrl::UnsupportedScheme(_)
                ),
                "expected UnsupportedScheme for {bad:?}"
            );
        }
    }

    #[test]
    fn parse_rejects_malformed() {
        assert!(matches!(
            BaseUrl::parse("not a url").unwrap_err(),
            InvalidBaseUrl::Malformed(_)
        ));
    }

    #[test]
    fn display_matches_as_str() {
        let base = BaseUrl::parse("https://api.anthropic.com").unwrap();
        assert_eq!(format!("{base}"), base.as_str());
    }
}
