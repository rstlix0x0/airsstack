//! Authentication scheme attached to every outgoing request.
//!
//! Lives in its own module so the set of accepted auth shapes evolves
//! independently of `Config` (which carries static request metadata) and of
//! the transport boundary (which is auth-agnostic). Adding a new variant
//! here is a localized change; downstream code that matches `Auth`
//! exhaustively will get a compile error pointing at the call site.
//!
//! Responsibilities:
//! - Define [`Auth`], the closed set of supported authentication schemes.
//! - Provide narrow accessors (e.g. [`Auth::api_key`]) so callers do not
//!   have to `match` on the enum for the common path.
//!
//! Not responsible for:
//! - Constructing the HTTP `x-api-key` header value — that lives in the
//!   header-construction layer.
//! - Validating the secret material — [`crate::types::ApiKey`] handles
//!   that at construction time.

use crate::types::ApiKey;

/// Closed set of supported authentication schemes.
///
/// Pattern matches against `Auth` should use a `_` arm so future additions
/// (for example, a `Bearer`-style variant for federated credentials) are a
/// non-breaking change for callers.
#[derive(Clone, Debug)]
pub enum Auth {
    /// Authenticate using an Anthropic API key sent in the `x-api-key` header.
    ApiKey(ApiKey),
}

impl Auth {
    /// Borrow the inner API key when the auth scheme is `ApiKey`.
    ///
    /// Returns `None` for any future non-API-key variant.
    #[must_use]
    pub const fn api_key(&self) -> Option<&ApiKey> {
        match self {
            Self::ApiKey(k) => Some(k),
        }
    }
}
