//! Plugin identity: the name its manifest declares and its cache directory uses.
//!
//! Responsibilities: [`PluginName`] and [`InvalidPluginName`]. Parsed at
//! construction; downstream code treats possession as proof that the value is
//! one safe path segment.

use crate::types::ident::is_segment;

/// A validated plugin name: non-empty, `[A-Za-z0-9._-]`, never `.` or `..`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct PluginName(String);

/// Why a string is not a usable plugin name.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid plugin name `{0}`: names are non-empty [A-Za-z0-9._-] path segments")]
pub struct InvalidPluginName(String);

impl PluginName {
    /// Parses a plugin name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPluginName`] when `raw` is empty, is `.` or `..`, or
    /// holds a character outside `[A-Za-z0-9._-]`.
    pub fn new(raw: impl Into<String>) -> Result<Self, InvalidPluginName> {
        let raw = raw.into();
        if is_segment(&raw, "._-") {
            Ok(Self(raw))
        } else {
            Err(InvalidPluginName(raw))
        }
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for PluginName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::PluginName;

    #[test]
    fn accepts_the_directory_shaped_names_plugins_use() {
        for ok in ["airsstack", "airsstack-guideline-rust", "minimal-plugin"] {
            assert!(PluginName::new(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_anything_that_would_not_be_one_path_segment() {
        for bad in ["", "a/b", "..", "a b", "café"] {
            assert!(PluginName::new(bad).is_err(), "{bad}");
        }
    }
}
