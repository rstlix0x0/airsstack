//! Authentication scheme attached to every outgoing request.
//!
//! Lives in its own module so the accepted auth shapes evolve independently
//! of `Config` (static request metadata) and the transport boundary
//! (auth-agnostic).
//!
//! Responsibilities:
//! - Define [`Auth`], the closed set of supported authentication schemes.
//! - Provide the [`Auth::api_key`] accessor for the common path.
//!
//! Not responsible for:
//! - Constructing the `Authorization` header value — the header layer does that.
//! - Validating secret material — [`crate::types::ApiKey`] handles that.

use crate::types::ApiKey;

/// Closed set of supported authentication schemes.
///
/// Match arms over `Auth` should use a `_` arm so future additions are a
/// non-breaking change for callers. `Debug` masks the inner key because
/// [`ApiKey`] masks its own secret.
#[derive(Clone, Debug)]
pub enum Auth {
    /// Authenticate with an OpenRouter API key sent as `Authorization: Bearer`.
    Bearer(ApiKey),
}

impl Auth {
    /// Borrow the inner API key when the scheme is `Bearer`.
    #[must_use]
    pub const fn api_key(&self) -> Option<&ApiKey> {
        match self {
            Self::Bearer(k) => Some(k),
        }
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
    fn bearer_exposes_api_key() {
        let key = ApiKey::new("sk-or-v1-abc").unwrap();
        let auth = Auth::Bearer(key);
        assert_eq!(
            auth.api_key().map(ApiKey::expose_secret),
            Some("sk-or-v1-abc")
        );
    }

    #[test]
    fn debug_does_not_leak_key() {
        let auth = Auth::Bearer(ApiKey::new("sk-or-secret").unwrap());
        assert!(!format!("{auth:?}").contains("secret"));
    }
}
