//! HTTP header names and canned values used when building requests against
//! the OpenRouter API.
//!
//! Crate-private. Caller-visible configuration goes through the strongly-typed
//! wrappers in `crate::types` and `crate::config`.
// `dead_code` fires on the lib target (no production callers yet); it does NOT
// fire on the test target (all constants appear in the test array). The lint
// fires conditionally across targets, so `#[expect]` would be reported
// "unfulfilled" on the test pass — `#[allow]` is the correct suppression for a
// conditionally-firing lint.
#![allow(dead_code)]
#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

pub(crate) const AUTHORIZATION: &str = "authorization";
pub(crate) const BEARER_PREFIX: &str = "Bearer ";
pub(crate) const CONTENT_TYPE: &str = "content-type";
pub(crate) const ACCEPT: &str = "accept";
pub(crate) const APPLICATION_JSON: &str = "application/json";
pub(crate) const TEXT_EVENT_STREAM: &str = "text/event-stream";
pub(crate) const HTTP_REFERER: &str = "http-referer";
pub(crate) const X_TITLE: &str = "x-title";
pub(crate) const RETRY_AFTER: &str = "retry-after";
pub(crate) const USER_AGENT: &str = "user-agent";
pub(crate) const X_OPENROUTER_CACHE: &str = "x-openrouter-cache";
pub(crate) const X_OPENROUTER_CACHE_TTL: &str = "x-openrouter-cache-ttl";
pub(crate) const X_OPENROUTER_CACHE_CLEAR: &str = "x-openrouter-cache-clear";
pub(crate) const X_OPENROUTER_CACHE_STATUS: &str = "x-openrouter-cache-status";
pub(crate) const X_OPENROUTER_CACHE_AGE: &str = "x-openrouter-cache-age";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_name_constants_are_lowercase_ascii() {
        for &h in &[
            AUTHORIZATION,
            CONTENT_TYPE,
            ACCEPT,
            APPLICATION_JSON,
            TEXT_EVENT_STREAM,
            HTTP_REFERER,
            X_TITLE,
            RETRY_AFTER,
            USER_AGENT,
            X_OPENROUTER_CACHE,
            X_OPENROUTER_CACHE_TTL,
            X_OPENROUTER_CACHE_CLEAR,
            X_OPENROUTER_CACHE_STATUS,
            X_OPENROUTER_CACHE_AGE,
        ] {
            assert!(
                h.bytes()
                    .all(|b| b.is_ascii_lowercase() || matches!(b, b'-' | b'/' | b'_')),
                "header name must be lowercase-ASCII with optional '-','/','_': got {h:?}"
            );
        }
    }

    #[test]
    fn bearer_prefix_has_trailing_space() {
        assert_eq!(BEARER_PREFIX, "Bearer ");
        assert!(BEARER_PREFIX.ends_with(' '));
    }
}
