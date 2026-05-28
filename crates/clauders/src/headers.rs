//! HTTP header names and canned values used when building requests against
//! the Anthropic API.
//!
//! Crate-private. Caller-visible header configuration goes through the
//! strongly-typed `AnthropicVersion` and `BetaHeader` wrappers in
//! `crate::types`.
// `dead_code` fires on the lib target (no production callers yet); it does NOT
// fire on the test target (all constants appear in the test array). Because the
// lint fires conditionally across targets, `#[expect]` would be reported
// "unfulfilled" by the test-target pass. Per M-LINT-OVERRIDE-EXPECT, `#[allow]`
// is the correct suppression for conditionally-firing lints.
#![allow(dead_code)]
#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

pub(crate) const X_API_KEY: &str = "x-api-key";
pub(crate) const ANTHROPIC_VERSION: &str = "anthropic-version";
pub(crate) const ANTHROPIC_BETA: &str = "anthropic-beta";
pub(crate) const CONTENT_TYPE: &str = "content-type";
pub(crate) const ACCEPT: &str = "accept";
pub(crate) const APPLICATION_JSON: &str = "application/json";
pub(crate) const TEXT_EVENT_STREAM: &str = "text/event-stream";
pub(crate) const REQUEST_ID: &str = "request-id";
pub(crate) const ANTHROPIC_ORG_ID: &str = "anthropic-organization-id";
pub(crate) const RETRY_AFTER: &str = "retry-after";
pub(crate) const USER_AGENT: &str = "user-agent";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_constants_are_lowercase_ascii() {
        for &h in &[
            X_API_KEY,
            ANTHROPIC_VERSION,
            ANTHROPIC_BETA,
            CONTENT_TYPE,
            ACCEPT,
            APPLICATION_JSON,
            TEXT_EVENT_STREAM,
            REQUEST_ID,
            ANTHROPIC_ORG_ID,
            RETRY_AFTER,
            USER_AGENT,
        ] {
            assert!(
                h.bytes()
                    .all(|b| b.is_ascii_lowercase() || matches!(b, b'-' | b'/' | b'_')),
                "header constant must be lowercase-ASCII with optional '-', '/', '_': got {h:?}"
            );
        }
    }
}
