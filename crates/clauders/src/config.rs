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

use crate::types::{AnthropicVersion, BetaHeader};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/";
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Static request configuration.
///
/// All fields are public so the builder layer can populate them directly
/// without going through setters. Construct via [`Config::default`] and
/// override the fields you care about, or use the client builder.
#[derive(Clone, Debug)]
pub struct Config {
    /// Base URL every request is built against. Defaults to
    /// `https://api.anthropic.com/`.
    pub base_url: url::Url,
    /// Value of the `anthropic-version` request header.
    pub anthropic_version: AnthropicVersion,
    /// Values joined into the `anthropic-beta` request header. Empty
    /// vector means the header is omitted entirely.
    pub anthropic_beta: Vec<BetaHeader>,
    /// Per-request wall-clock timeout applied by the transport layer.
    pub timeout: Duration,
}

impl Default for Config {
    #[expect(
        clippy::expect_used,
        reason = "DEFAULT_BASE_URL is a compile-time constant that is always a valid absolute URL"
    )]
    fn default() -> Self {
        Self {
            base_url: url::Url::parse(DEFAULT_BASE_URL)
                .expect("invariant: DEFAULT_BASE_URL is a valid absolute URL"),
            anthropic_version: AnthropicVersion::default(),
            anthropic_beta: Vec::new(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn default_points_at_anthropic() {
        let c = Config::default();
        assert_eq!(c.base_url.as_str(), DEFAULT_BASE_URL);
        assert_eq!(c.anthropic_version, AnthropicVersion::default());
        assert!(c.anthropic_beta.is_empty());
        assert_eq!(c.timeout, Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    }
}
