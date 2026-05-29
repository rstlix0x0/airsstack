//! Static request configuration carried by every `Client`.
//!
//! Lives apart from [`crate::auth::Auth`] so the credential and the
//! non-secret request metadata can be reasoned about and tested
//! independently. `Config` is cheap to clone, has sensible defaults, and is
//! intended to be constructed via the client builder rather than directly.
//!
//! Responsibilities:
//! - Declare [`Config`], holding base URL, Anthropic API version, beta
//!   header set, and per-request timeout.
//! - Provide [`Config::default`] pointing at the production Anthropic
//!   endpoint with the SDK's default version and a sixty-second timeout.
//!
//! Not responsible for:
//! - Authentication material (see [`crate::auth::Auth`]).
//! - Retry / backoff policy — that belongs in a dedicated retry module.
//! - HTTP header construction — `Config` carries the values; the
//!   header-builder layer formats them on the wire.

use std::time::Duration;

use crate::types::{AnthropicVersion, BaseUrl, BetaHeader};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/";
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Static request configuration.
///
/// Fields are crate-private so the only ways to set them are
/// [`Config::default`] and the client builder; this prevents a caller from
/// mutating a live `Config` out from under the builder's validation. Read
/// them back through the accessor methods.
#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) base_url: BaseUrl,
    pub(crate) anthropic_version: AnthropicVersion,
    pub(crate) anthropic_beta: Vec<BetaHeader>,
    pub(crate) timeout: Duration,
}

impl Config {
    /// Base URL every request is built against. Defaults to
    /// `https://api.anthropic.com/`.
    #[must_use]
    pub const fn base_url(&self) -> &BaseUrl {
        &self.base_url
    }

    /// Value of the `anthropic-version` request header.
    #[must_use]
    pub const fn anthropic_version(&self) -> &AnthropicVersion {
        &self.anthropic_version
    }

    /// Values joined into the `anthropic-beta` request header. An empty
    /// slice means the header is omitted entirely.
    #[must_use]
    pub fn anthropic_beta(&self) -> &[BetaHeader] {
        &self.anthropic_beta
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
        reason = "DEFAULT_BASE_URL is a compile-time constant that is always a valid http(s) URL"
    )]
    fn default() -> Self {
        Self {
            base_url: BaseUrl::parse(DEFAULT_BASE_URL)
                .expect("invariant: DEFAULT_BASE_URL is a valid http(s) URL"),
            anthropic_version: AnthropicVersion::default(),
            anthropic_beta: Vec::new(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_points_at_anthropic() {
        let c = Config::default();
        assert_eq!(c.base_url().as_str(), DEFAULT_BASE_URL);
        assert_eq!(c.anthropic_version(), &AnthropicVersion::default());
        assert!(c.anthropic_beta().is_empty());
        assert_eq!(c.timeout(), Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    }
}
