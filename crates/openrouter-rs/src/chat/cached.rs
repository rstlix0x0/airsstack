//! Generic envelope pairing a decoded value with its edge-cache outcome.
//!
//! Exists as its own file because the envelope is generic over the payload
//! (`Cached<ChatCompletion>` for `send_cached`, `Cached<ChatStream>` for
//! `stream_cached`) and is the read-side counterpart to the request-side
//! `response_cache.rs` control.
//!
//! Responsibilities:
//! - [`Cached`] — the value + cache-outcome envelope.
//! - [`CacheStatus`] — whether the response was a cache hit or miss.

/// Whether a response was served from the gateway edge cache.
///
/// Decoded from `X-OpenRouter-Cache-Status`. An absent or unrecognized header
/// is treated as [`CacheStatus::Miss`].
///
/// # Examples
/// ```
/// use openrouter_rs::chat::CacheStatus;
/// assert_eq!(CacheStatus::from_header_value("HIT"), CacheStatus::Hit);
/// assert_eq!(CacheStatus::from_header_value("MISS"), CacheStatus::Miss);
/// assert_eq!(CacheStatus::from_header_value("anything-else"), CacheStatus::Miss);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheStatus {
    /// Served from the edge cache.
    Hit,
    /// Computed fresh (or cache disabled / header absent).
    Miss,
}

impl CacheStatus {
    /// Parse a `X-OpenRouter-Cache-Status` header value. Case-insensitive;
    /// anything other than `HIT` maps to [`CacheStatus::Miss`].
    #[must_use]
    pub fn from_header_value(value: &str) -> Self {
        if value.eq_ignore_ascii_case("HIT") {
            Self::Hit
        } else {
            Self::Miss
        }
    }
}

/// A decoded value paired with its edge-cache outcome.
///
/// Returned by [`crate::chat::ChatResource::send_cached`] and
/// [`crate::chat::ChatResource::stream_cached`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cached<T> {
    /// The decoded payload (e.g. a `ChatCompletion` or a `ChatStream`).
    pub value: T,
    /// Whether the response was a cache hit.
    pub status: CacheStatus,
    /// Age of the cached entry in seconds (`X-OpenRouter-Cache-Age`, hit only).
    pub age_secs: Option<u32>,
    /// Remaining/declared TTL in seconds (`X-OpenRouter-Cache-TTL`).
    pub ttl_secs: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_parses_hit_case_insensitively() {
        assert_eq!(CacheStatus::from_header_value("HIT"), CacheStatus::Hit);
        assert_eq!(CacheStatus::from_header_value("hit"), CacheStatus::Hit);
    }

    #[test]
    fn status_maps_everything_else_to_miss() {
        assert_eq!(CacheStatus::from_header_value("MISS"), CacheStatus::Miss);
        assert_eq!(CacheStatus::from_header_value(""), CacheStatus::Miss);
        assert_eq!(CacheStatus::from_header_value("weird"), CacheStatus::Miss);
    }

    #[test]
    fn envelope_holds_value_and_outcome() {
        let c = Cached {
            value: 42_u32,
            status: CacheStatus::Hit,
            age_secs: Some(10),
            ttl_secs: Some(300),
        };
        assert_eq!(c.value, 42);
        assert_eq!(c.status, CacheStatus::Hit);
        assert_eq!(c.age_secs, Some(10));
        assert_eq!(c.ttl_secs, Some(300));
    }
}
