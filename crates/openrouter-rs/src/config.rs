//! Static request configuration carried by every `Client`.
//!
//! Lives apart from [`crate::auth::Auth`] so the credential and the non-secret
//! request metadata are reasoned about and tested independently. `Config` is
//! cheap to clone, has sensible defaults, and is built via the client builder
//! rather than directly.
//!
//! Responsibilities:
//! - Declare [`Config`] (base URL, app-attribution headers, per-request timeout).
//! - Provide [`Config::default`] pointing at the OpenRouter production endpoint.
//!
//! Not responsible for:
//! - Authentication material (see [`crate::auth::Auth`]).
//! - Retry / backoff policy.
//! - HTTP header construction — `Config` carries the values; the header layer
//!   formats them on the wire.

use std::time::Duration;

use crate::types::BaseUrl;

const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1/";
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Static request configuration.
///
/// Fields are crate-private so the only ways to set them are
/// [`Config::default`] and the client builder; read them back through the
/// accessors.
#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) base_url: BaseUrl,
    pub(crate) http_referer: Option<String>,
    pub(crate) app_title: Option<String>,
    pub(crate) timeout: Duration,
}

impl Config {
    /// Base URL every request is built against. Defaults to
    /// `https://openrouter.ai/api/v1/` (trailing slash for additive joins).
    #[must_use]
    pub const fn base_url(&self) -> &BaseUrl {
        &self.base_url
    }

    /// Value of the optional `HTTP-Referer` attribution header.
    #[must_use]
    pub fn http_referer(&self) -> Option<&str> {
        self.http_referer.as_deref()
    }

    /// Value of the optional `X-Title` attribution header.
    #[must_use]
    pub fn app_title(&self) -> Option<&str> {
        self.app_title.as_deref()
    }

    /// Per-request wall-clock timeout applied by the transport layer.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Default for Config {
    #[expect(
        clippy::expect_used,
        reason = "DEFAULT_BASE_URL is a compile-time constant that is always a valid https URL"
    )]
    fn default() -> Self {
        Self {
            base_url: BaseUrl::parse(DEFAULT_BASE_URL)
                .expect("invariant: DEFAULT_BASE_URL is a valid https URL"),
            http_referer: None,
            app_title: None,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_points_at_openrouter() {
        let c = Config::default();
        assert_eq!(c.base_url().as_str(), DEFAULT_BASE_URL);
        assert_eq!(c.timeout(), Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        assert!(c.http_referer().is_none());
        assert!(c.app_title().is_none());
    }
}
