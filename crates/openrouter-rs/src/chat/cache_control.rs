//! Provider prompt-cache control attached to request-body content.
//!
//! Exists as its own file because the prompt cache is a request-body concern
//! distinct from the response edge-cache (`response_cache.rs`, HTTP headers) and
//! from the cache usage stats (`token_details.rs`, response body). The single
//! [`CacheControl`] type is rendered at two request-body attach points: inside a
//! message content part, and at the top level of a request.
//!
//! Responsibilities:
//! - [`CacheControl`] — the `cache_control` wire object.
//! - [`CacheKind`] — the cache discriminator (`ephemeral` is the only value).
//! - [`CacheTtl`] — the optional time-to-live (`5m` / `1h`).
//!
//! Not responsible for the gateway edge cache (response headers) — see
//! `response_cache.rs`.

use serde::{Deserialize, Serialize};

/// The kind of cache breakpoint. Only `ephemeral` is documented.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::CacheKind;
/// assert_eq!(serde_json::to_value(CacheKind::Ephemeral).unwrap(), "ephemeral");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheKind {
    /// A short-lived provider cache entry.
    Ephemeral,
}

/// Cache time-to-live. Absent means the provider default (5 minutes).
///
/// # Examples
/// ```
/// use openrouter_rs::chat::CacheTtl;
/// assert_eq!(serde_json::to_value(CacheTtl::FiveMinutes).unwrap(), "5m");
/// assert_eq!(serde_json::to_value(CacheTtl::OneHour).unwrap(), "1h");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheTtl {
    /// Five-minute TTL (`"5m"`).
    #[serde(rename = "5m")]
    FiveMinutes,
    /// One-hour TTL (`"1h"`).
    #[serde(rename = "1h")]
    OneHour,
}

/// A `cache_control` breakpoint marking cacheable request content.
///
/// Construct with [`CacheControl::ephemeral`] (provider-default TTL) or
/// [`CacheControl::with_ttl`]. Serializes to `{ "type": "ephemeral" }` or
/// `{ "type": "ephemeral", "ttl": "1h" }`.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::{CacheControl, CacheTtl};
/// assert_eq!(
///     serde_json::to_value(CacheControl::ephemeral()).unwrap(),
///     serde_json::json!({ "type": "ephemeral" }),
/// );
/// assert_eq!(
///     serde_json::to_value(CacheControl::with_ttl(CacheTtl::OneHour)).unwrap(),
///     serde_json::json!({ "type": "ephemeral", "ttl": "1h" }),
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheControl {
    /// The cache discriminator. Always [`CacheKind::Ephemeral`] today.
    #[serde(rename = "type")]
    pub kind: CacheKind,
    /// Optional TTL; omitted from the wire when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<CacheTtl>,
}

impl CacheControl {
    /// An ephemeral breakpoint with provider-default TTL.
    #[must_use]
    pub const fn ephemeral() -> Self {
        Self {
            kind: CacheKind::Ephemeral,
            ttl: None,
        }
    }

    /// An ephemeral breakpoint with an explicit TTL.
    #[must_use]
    pub const fn with_ttl(ttl: CacheTtl) -> Self {
        Self {
            kind: CacheKind::Ephemeral,
            ttl: Some(ttl),
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
    use serde_json::json;

    #[test]
    fn ephemeral_serializes_without_ttl() {
        assert_eq!(
            serde_json::to_value(CacheControl::ephemeral()).unwrap(),
            json!({ "type": "ephemeral" }),
        );
    }

    #[test]
    fn with_ttl_serializes_both_ttls() {
        assert_eq!(
            serde_json::to_value(CacheControl::with_ttl(CacheTtl::FiveMinutes)).unwrap(),
            json!({ "type": "ephemeral", "ttl": "5m" }),
        );
        assert_eq!(
            serde_json::to_value(CacheControl::with_ttl(CacheTtl::OneHour)).unwrap(),
            json!({ "type": "ephemeral", "ttl": "1h" }),
        );
    }

    #[test]
    fn kind_and_ttl_serialize_to_wire_strings() {
        assert_eq!(
            serde_json::to_value(CacheKind::Ephemeral).unwrap(),
            json!("ephemeral")
        );
        assert_eq!(
            serde_json::to_value(CacheTtl::FiveMinutes).unwrap(),
            json!("5m")
        );
        assert_eq!(
            serde_json::to_value(CacheTtl::OneHour).unwrap(),
            json!("1h")
        );
    }
}
