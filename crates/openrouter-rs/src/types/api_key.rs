//! Opaque, secret-protected API key wrapper.

use secrecy::{ExposeSecret, SecretString};

/// API key for authenticating against the OpenRouter API.
///
/// The inner secret is wrapped in [`SecretString`], so `Debug` prints
/// `"ApiKey(\"***\")"` instead of the raw key. Use [`ApiKey::expose_secret`]
/// to obtain the value when building the `Authorization: Bearer` header.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::ApiKey;
/// let key = ApiKey::new("sk-or-v1-abcdef").expect("valid key");
/// assert_eq!(key.expose_secret(), "sk-or-v1-abcdef");
/// let dbg = format!("{key:?}");
/// assert!(!dbg.contains("sk-or"));
/// ```
///
/// ```
/// use openrouter_rs::types::ApiKey;
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

    /// Borrow the raw key for sending in the `Authorization` header.
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
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(ApiKey::new("").unwrap_err(), InvalidApiKey::Empty);
    }

    #[test]
    fn rejects_whitespace_and_non_ascii() {
        assert_eq!(ApiKey::new("a b").unwrap_err(), InvalidApiKey::NonPrintable);
        assert_eq!(
            ApiKey::new("a\tb").unwrap_err(),
            InvalidApiKey::NonPrintable
        );
        assert_eq!(
            ApiKey::new("sk-héllo").unwrap_err(),
            InvalidApiKey::NonPrintable
        );
    }

    #[test]
    fn accepts_typical_openrouter_key() {
        let k = ApiKey::new("sk-or-v1-abcdef0123456789").unwrap();
        assert!(k.expose_secret().starts_with("sk-or-"));
    }

    #[test]
    fn debug_masks_secret() {
        let k = ApiKey::new("sk-or-secret-1234").unwrap();
        let dbg = format!("{k:?}");
        assert_eq!(dbg, "ApiKey(\"***\")");
        assert!(!dbg.contains("secret"));
    }
}
