//! Marketplace identity: the name a marketplace.json declares and its cache
//! directory uses.
//!
//! Responsibilities: [`MarketplaceName`] and [`InvalidMarketplaceName`]. Parsed
//! at construction; downstream code treats possession as proof that the value
//! is one safe path segment.

use crate::types::ident::is_segment;

/// A validated marketplace name: non-empty, `[A-Za-z0-9._-]`, never `.` or `..`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct MarketplaceName(String);

/// Why a string is not a usable marketplace name.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid marketplace name `{0}`: names are non-empty [A-Za-z0-9._-] path segments")]
pub struct InvalidMarketplaceName(String);

impl MarketplaceName {
    /// Parses a marketplace name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidMarketplaceName`] when `raw` is empty, is `.` or `..`,
    /// or holds a character outside `[A-Za-z0-9._-]`.
    pub fn new(raw: impl Into<String>) -> Result<Self, InvalidMarketplaceName> {
        let raw = raw.into();
        if is_segment(&raw, "._-") {
            Ok(Self(raw))
        } else {
            Err(InvalidMarketplaceName(raw))
        }
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for MarketplaceName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::MarketplaceName;

    #[test]
    fn accepts_this_repositorys_marketplace_name() {
        assert!(MarketplaceName::new("airsstack").is_ok());
        assert!(MarketplaceName::new("claude-plugins-official").is_ok());
    }

    #[test]
    fn rejects_a_traversal_segment() {
        assert!(MarketplaceName::new("..").is_err());
    }
}
