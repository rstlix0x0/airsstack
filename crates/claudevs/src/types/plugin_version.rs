//! Plugin version identity: the version segment the install cache uses as a
//! directory name.
//!
//! Responsibilities: [`PluginVersion`] and [`InvalidPluginVersion`]. Parsed at
//! construction; downstream code treats possession as proof that the value is
//! one safe path segment.

use crate::types::ident::is_segment;

/// A validated plugin version: non-empty, `[A-Za-z0-9.+-]`, never `.` or `..`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct PluginVersion(String);

/// Why a string is not a usable plugin version.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid plugin version `{0}`: versions are non-empty [A-Za-z0-9.+-] path segments")]
pub struct InvalidPluginVersion(String);

impl PluginVersion {
    /// Parses a plugin version.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPluginVersion`] when `raw` is empty, is `.` or `..`, or
    /// holds a character outside `[A-Za-z0-9.+-]`.
    pub fn new(raw: impl Into<String>) -> Result<Self, InvalidPluginVersion> {
        let raw = raw.into();
        if is_segment(&raw, ".+-") {
            Ok(Self(raw))
        } else {
            Err(InvalidPluginVersion(raw))
        }
    }

    /// The version as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::PluginVersion;

    #[test]
    fn accepts_the_versions_the_install_cache_uses_as_directories() {
        for ok in ["0.1.0", "1.0.0", "2.1.0-beta.1"] {
            assert!(PluginVersion::new(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_a_version_that_would_escape_the_cache_root() {
        assert!(PluginVersion::new("../../etc").is_err());
        assert!(PluginVersion::new("..").is_err());
    }

    /// `Path::new("/cache/a/p").join("../../../etc")` does not normalize —
    /// the joined path is literally `/cache/a/p/../../../etc`, with every
    /// `..` component surviving in the result rather than being resolved
    /// away — so nothing downstream of a plain `Path::join` stops those
    /// components from walking back out of the cache root once the OS
    /// resolves them. Rejecting a version shaped like this at construction is
    /// therefore not a convenience check: it is the only place in the
    /// pipeline where the traversal is actually stopped.
    #[test]
    fn rejecting_the_version_is_what_stops_a_cache_join_from_escaping() {
        let escaping_raw = "../../../etc";
        let joined = std::path::Path::new("/cache/marketplace/plugin").join(escaping_raw);
        assert!(
            joined
                .components()
                .any(|c| c == std::path::Component::ParentDir),
            "join must leave `..` unresolved for this test to demonstrate the risk it names"
        );
        assert!(
            PluginVersion::new(escaping_raw).is_err(),
            "PluginVersion must reject exactly the value that an unresolved join would carry \
             straight through to the filesystem"
        );
    }
}
