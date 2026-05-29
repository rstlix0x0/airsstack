//! Prompt-caching control values for the Messages API.
//!
//! Anthropic supports a single cache tier (`ephemeral`) with an optional
//! time-to-live. The enum is `#[non_exhaustive]` so future tiers can be added
//! as non-breaking variants.
//!
//! Responsibilities:
//! - Define [`CacheControl`], the cache breakpoint marker placed on
//!   cacheable request components.
//! - Define [`CacheTtl`], the optional TTL that selects between the 5-minute
//!   and 1-hour ephemeral cache tiers.
//!
//! Not responsible for:
//! - Attaching `CacheControl` to specific request types — that lives in the
//!   respective type definitions under `messages/` and `types/system.rs`.

/// Cache time-to-live tier for an ephemeral cache breakpoint.
///
/// When omitted, the API defaults to the 5-minute tier.
/// The 1-hour tier (`OneHour`) stores content longer at 2× the write price.
///
/// See the [Anthropic prompt-caching docs](https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching)
/// for pricing and availability details.
///
/// # Examples
///
/// ```
/// use clauders::types::CacheTtl;
/// let j = serde_json::to_string(&CacheTtl::OneHour).unwrap();
/// assert_eq!(j, r#""1h""#);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CacheTtl {
    /// 5-minute TTL (default when `ttl` is omitted).
    #[serde(rename = "5m")]
    FiveMinutes,
    /// 1-hour TTL — extended duration at 2× cache-write price.
    #[serde(rename = "1h")]
    OneHour,
}

/// Cache breakpoint marker for cacheable request components.
///
/// Attach to a [`crate::types::SystemSegment`], [`crate::messages::TextBlock`],
/// [`crate::messages::tools::Tool`], or a tool-result block to mark that
/// component as the cache breakpoint for prompt caching.
///
/// The enum is `#[non_exhaustive]` so future cache tiers can be added in a
/// non-breaking minor release.
///
/// See the [Anthropic prompt-caching docs](https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching)
/// for the full carrier set and minimum token requirements.
///
/// # Examples
///
/// Ephemeral cache with default TTL:
///
/// ```
/// use clauders::types::CacheControl;
/// let cc = CacheControl::ephemeral();
/// let j = serde_json::to_string(&cc).unwrap();
/// assert_eq!(j, r#"{"type":"ephemeral"}"#);
/// ```
///
/// Ephemeral cache with explicit 1-hour TTL:
///
/// ```
/// use clauders::types::{CacheControl, CacheTtl};
/// let cc = CacheControl::ephemeral_with_ttl(CacheTtl::OneHour);
/// let j = serde_json::to_string(&cc).unwrap();
/// assert_eq!(j, r#"{"type":"ephemeral","ttl":"1h"}"#);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CacheControl {
    /// Ephemeral cache tier with an optional TTL.
    ///
    /// When `ttl` is `None`, the API applies the default 5-minute TTL.
    Ephemeral {
        /// Optional cache duration; `None` uses the API default (5 minutes).
        #[serde(skip_serializing_if = "Option::is_none", default)]
        ttl: Option<CacheTtl>,
    },
}

impl CacheControl {
    /// Construct an ephemeral cache breakpoint using the API default TTL (5 minutes).
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::types::CacheControl;
    /// let cc = CacheControl::ephemeral();
    /// let j = serde_json::to_string(&cc).unwrap();
    /// assert_eq!(j, r#"{"type":"ephemeral"}"#);
    /// ```
    #[must_use]
    pub const fn ephemeral() -> Self {
        Self::Ephemeral { ttl: None }
    }

    /// Construct an ephemeral cache breakpoint with an explicit TTL.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::types::{CacheControl, CacheTtl};
    /// let cc = CacheControl::ephemeral_with_ttl(CacheTtl::OneHour);
    /// let j = serde_json::to_string(&cc).unwrap();
    /// assert_eq!(j, r#"{"type":"ephemeral","ttl":"1h"}"#);
    /// ```
    #[must_use]
    pub const fn ephemeral_with_ttl(ttl: CacheTtl) -> Self {
        Self::Ephemeral { ttl: Some(ttl) }
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
    fn ephemeral_wire_format_no_ttl() {
        let c = CacheControl::ephemeral();
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(j, r#"{"type":"ephemeral"}"#);
    }

    #[test]
    fn ephemeral_wire_format_with_1h_ttl() {
        let c = CacheControl::ephemeral_with_ttl(CacheTtl::OneHour);
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(j, r#"{"type":"ephemeral","ttl":"1h"}"#);
    }

    #[test]
    fn ephemeral_wire_format_with_5m_ttl() {
        let c = CacheControl::ephemeral_with_ttl(CacheTtl::FiveMinutes);
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(j, r#"{"type":"ephemeral","ttl":"5m"}"#);
    }

    #[test]
    fn ephemeral_no_ttl_round_trips() {
        let original = CacheControl::ephemeral();
        let j = serde_json::to_string(&original).unwrap();
        let back: CacheControl = serde_json::from_str(&j).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn ephemeral_with_ttl_round_trips() {
        let original = CacheControl::ephemeral_with_ttl(CacheTtl::OneHour);
        let j = serde_json::to_string(&original).unwrap();
        let back: CacheControl = serde_json::from_str(&j).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn cache_ttl_variants_serialize_correctly() {
        assert_eq!(
            serde_json::to_string(&CacheTtl::FiveMinutes).unwrap(),
            r#""5m""#
        );
        assert_eq!(
            serde_json::to_string(&CacheTtl::OneHour).unwrap(),
            r#""1h""#
        );
    }
}
