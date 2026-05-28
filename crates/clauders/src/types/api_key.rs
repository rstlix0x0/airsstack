//! Opaque, secret-protected API key wrapper.

use secrecy::{ExposeSecret, SecretString};

/// API key for authenticating against the Anthropic API.
///
/// The inner secret string is wrapped in [`SecretString`], so `Debug`
/// output prints `"ApiKey(\"***\")"` instead of the raw key. Use
/// [`ApiKey::expose_secret`] to obtain the underlying value when sending
/// it as the `x-api-key` header value.
///
/// # Examples
///
/// ```
/// use clauders::types::ApiKey;
/// let key = ApiKey::new("sk-test-abcdef").expect("valid key");
/// assert_eq!(key.expose_secret(), "sk-test-abcdef");
/// // Debug never prints the secret:
/// let dbg = format!("{key:?}");
/// assert!(!dbg.contains("sk-test"));
/// ```
///
/// ```
/// use clauders::types::ApiKey;
/// assert!(ApiKey::new("").is_err());
/// assert!(ApiKey::new("with space").is_err());
/// ```
#[derive(Clone)]
pub struct ApiKey(SecretString);

/// Reasons [`ApiKey::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidApiKey {
    /// The provided string was empty.
    #[error("API key must not be empty")]
    Empty,
    /// The provided string contained non-ASCII or whitespace bytes.
    #[error("API key must be ASCII printable without whitespace")]
    NonPrintable,
}

impl ApiKey {
    /// Validate and wrap a string as an `ApiKey`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidApiKey::Empty`] if `raw` is empty, or
    /// [`InvalidApiKey::NonPrintable`] if it contains non-ASCII or
    /// whitespace characters.
    pub fn new(raw: impl Into<String>) -> Result<Self, InvalidApiKey> {
        let s: String = raw.into();
        if s.is_empty() {
            return Err(InvalidApiKey::Empty);
        }
        if !s.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(InvalidApiKey::NonPrintable);
        }
        Ok(Self(SecretString::from(s)))
    }

    /// Borrow the raw key for sending in HTTP headers.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ApiKey").field(&"***").finish()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(ApiKey::new("").unwrap_err(), InvalidApiKey::Empty);
    }

    #[test]
    fn rejects_whitespace() {
        assert_eq!(ApiKey::new("a b").unwrap_err(), InvalidApiKey::NonPrintable);
        assert_eq!(
            ApiKey::new("a\tb").unwrap_err(),
            InvalidApiKey::NonPrintable
        );
        assert_eq!(
            ApiKey::new("a\nb").unwrap_err(),
            InvalidApiKey::NonPrintable
        );
    }

    #[test]
    fn rejects_non_ascii() {
        assert_eq!(
            ApiKey::new("sk-héllo").unwrap_err(),
            InvalidApiKey::NonPrintable
        );
    }

    #[test]
    fn accepts_typical_anthropic_key() {
        let k = ApiKey::new("sk-ant-api03-abcdef0123456789").unwrap();
        assert!(k.expose_secret().starts_with("sk-ant-"));
    }

    #[test]
    fn debug_masks_secret() {
        let k = ApiKey::new("sk-secret-key-1234").unwrap();
        let dbg = format!("{k:?}");
        assert_eq!(dbg, "ApiKey(\"***\")");
        assert!(!dbg.contains("secret"));
    }
}
