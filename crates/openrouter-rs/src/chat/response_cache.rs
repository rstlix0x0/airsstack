//! Per-request control for the OpenRouter gateway edge cache.
//!
//! Exists as its own file because the edge cache is an HTTP-header concern,
//! distinct from the request-body prompt cache (`cache_control.rs`). This type
//! is NOT serialized into the request body; the resource layer reads it and
//! renders the three `X-OpenRouter-Cache*` request headers from it.
//!
//! Responsibilities:
//! - [`ResponseCache`] — the request control + its fluent builder.
//! - [`CacheMode`] — cache on/off.
//! - [`CacheClear`] — whether to force-refresh the cache entry.
//! - [`CacheTtlSeconds`] — validated TTL in seconds (`1..=86400`).
//!
//! Not responsible for reading the response cache headers — that produces a
//! [`crate::chat::Cached`] envelope; see `cached.rs`.

/// Whether the gateway edge cache is enabled for a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheMode {
    /// Use the edge cache (`X-OpenRouter-Cache: true`).
    Enabled,
    /// Bypass the edge cache (`X-OpenRouter-Cache: false`).
    Disabled,
}

/// Whether to force-refresh (clear) the cache entry for a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheClear {
    /// Clear the entry and recompute (`X-OpenRouter-Cache-Clear: true`).
    Clear,
    /// Leave any existing entry in place (no clear header sent).
    Keep,
}

/// Reason [`CacheTtlSeconds::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("cache TTL must be within 1..=86400 seconds")]
pub struct InvalidCacheTtlSeconds;

/// Edge-cache TTL in seconds. Range `1..=86400` (1 second to 1 day).
///
/// # Examples
/// ```
/// use openrouter_rs::chat::CacheTtlSeconds;
/// assert_eq!(CacheTtlSeconds::new(600).unwrap().get(), 600);
/// assert!(CacheTtlSeconds::new(0).is_err());
/// assert!(CacheTtlSeconds::new(86_401).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacheTtlSeconds(u32);

impl CacheTtlSeconds {
    /// Validate and wrap a seconds value.
    ///
    /// # Errors
    /// Returns [`InvalidCacheTtlSeconds`] when `secs` is `0` or above `86_400`.
    pub const fn new(secs: u32) -> Result<Self, InvalidCacheTtlSeconds> {
        if secs == 0 || secs > 86_400 {
            return Err(InvalidCacheTtlSeconds);
        }
        Ok(Self(secs))
    }

    /// The inner value, for header rendering.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Per-request edge-cache control.
///
/// Build from [`ResponseCache::enabled`] or [`ResponseCache::disabled`], then
/// optionally chain [`ResponseCache::ttl_secs`] and [`ResponseCache::clear`].
/// Pass it to [`crate::chat::ChatResource::send_cached`] or
/// [`crate::chat::ChatResource::stream_cached`].
///
/// # Examples
/// ```
/// use openrouter_rs::chat::{CacheMode, ResponseCache};
/// let rc = ResponseCache::enabled().ttl_secs(600).unwrap().clear();
/// assert_eq!(rc.mode(), CacheMode::Enabled);
/// assert_eq!(rc.ttl().unwrap().get(), 600);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResponseCache {
    mode: CacheMode,
    ttl: Option<CacheTtlSeconds>,
    clear: CacheClear,
}

impl ResponseCache {
    /// A control with the edge cache enabled, no explicit TTL, no clear.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            mode: CacheMode::Enabled,
            ttl: None,
            clear: CacheClear::Keep,
        }
    }

    /// A control with the edge cache disabled.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            mode: CacheMode::Disabled,
            ttl: None,
            clear: CacheClear::Keep,
        }
    }

    /// Set the cache TTL in seconds.
    ///
    /// # Errors
    /// Returns [`InvalidCacheTtlSeconds`] when `secs` is outside `1..=86400`.
    pub const fn ttl_secs(mut self, secs: u32) -> Result<Self, InvalidCacheTtlSeconds> {
        match CacheTtlSeconds::new(secs) {
            Ok(t) => {
                self.ttl = Some(t);
                Ok(self)
            }
            Err(e) => Err(e),
        }
    }

    /// Force-refresh the cache entry for this request.
    #[must_use]
    pub const fn clear(mut self) -> Self {
        self.clear = CacheClear::Clear;
        self
    }

    /// The cache mode.
    #[must_use]
    pub const fn mode(self) -> CacheMode {
        self.mode
    }

    /// The configured TTL, if any.
    #[must_use]
    pub const fn ttl(self) -> Option<CacheTtlSeconds> {
        self.ttl
    }

    /// The clear directive.
    #[must_use]
    pub const fn clear_directive(self) -> CacheClear {
        self.clear
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
    fn ttl_seconds_bounds() {
        assert!(CacheTtlSeconds::new(0).is_err());
        assert_eq!(CacheTtlSeconds::new(1).unwrap().get(), 1);
        assert_eq!(CacheTtlSeconds::new(86_400).unwrap().get(), 86_400);
        assert!(CacheTtlSeconds::new(86_401).is_err());
    }

    #[test]
    fn enabled_defaults() {
        let rc = ResponseCache::enabled();
        assert_eq!(rc.mode(), CacheMode::Enabled);
        assert_eq!(rc.ttl(), None);
        assert_eq!(rc.clear_directive(), CacheClear::Keep);
    }

    #[test]
    fn disabled_defaults() {
        let rc = ResponseCache::disabled();
        assert_eq!(rc.mode(), CacheMode::Disabled);
    }

    #[test]
    fn builder_chain_sets_ttl_and_clear() {
        let rc = ResponseCache::enabled().ttl_secs(600).unwrap().clear();
        assert_eq!(rc.mode(), CacheMode::Enabled);
        assert_eq!(rc.ttl().unwrap().get(), 600);
        assert_eq!(rc.clear_directive(), CacheClear::Clear);
    }

    #[test]
    fn ttl_secs_rejects_out_of_range() {
        assert_eq!(
            ResponseCache::enabled().ttl_secs(0).unwrap_err(),
            InvalidCacheTtlSeconds
        );
    }
}
