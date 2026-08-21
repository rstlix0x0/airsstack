//! Reading the two manifests the install-cache path is built from.
//!
//! The cache directory a plugin installs into is
//! `cache/<marketplace>/<plugin>/<version>/` — verified against this machine's
//! own registry, where `installPath` values have exactly that shape. The
//! marketplace name comes from the repository that *hosts* the plugin, so it is
//! found by walking up from the plugin directory; the plugin name and version
//! come from the plugin's own manifest. Neither is ever hardcoded: the engine
//! carries no knowledge of any particular marketplace.
//!
//! Responsibilities: [`PluginManifest`], [`read`], [`marketplace_name`].

use std::path::Path;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::types::{MarketplaceName, PluginName, PluginVersion};

/// What a plugin's own manifest contributes to its install path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifest {
    /// The plugin's name.
    pub name: PluginName,
    /// The version its cache directory is keyed by.
    pub version: PluginVersion,
}

/// Reads `<plugin>/.claude-plugin/plugin.json`.
///
/// # Errors
///
/// [`Error::Manifest`] when the file is missing, is not JSON, or lacks a usable
/// `name` or `version`.
pub fn read(plugin_dir: &Path) -> Result<PluginManifest> {
    let path = plugin_dir.join(".claude-plugin/plugin.json");
    let manifest = |reason: String| Error::Manifest {
        path: path.display().to_string(),
        reason,
    };

    let text = std::fs::read_to_string(&path)
        .map_err(|source| manifest(format!("cannot read plugin.json: {source}")))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|source| manifest(format!("not JSON: {source}")))?;

    let name = value
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| manifest(String::from("no `name` field")))?;
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| manifest(String::from("no `version` field")))?;

    Ok(PluginManifest {
        name: PluginName::new(name).map_err(|e| manifest(e.to_string()))?,
        version: PluginVersion::new(version).map_err(|e| manifest(e.to_string()))?,
    })
}

/// The ancestors of `plugin_dir`, up to and including the repository root.
///
/// A marketplace belongs to one repository, so the search for it stops at the
/// first ancestor holding a `.git` entry. Without that floor the walk runs to
/// `/`, and a plugin whose own repository declares no marketplace would be
/// keyed by a stray manifest in some unrelated parent directory — which is a
/// real arrangement here, where a worktree lives beneath its parent checkout.
fn bounded_ancestors(plugin_dir: &Path) -> impl Iterator<Item = &Path> {
    let mut stop = false;
    plugin_dir.ancestors().take_while(move |ancestor| {
        if stop {
            return false;
        }
        // `.git` is a directory in a checkout and a file in a linked worktree.
        stop = ancestor.join(".git").exists();
        true
    })
}

/// The marketplace name of the repository hosting `plugin_dir`.
///
/// # Errors
///
/// [`Error::Marketplace`] when no ancestor holds a
/// `.claude-plugin/marketplace.json` — a placement gap, not a plugin defect.
/// [`Error::Manifest`] when the one found is not JSON or has no usable `name`.
pub fn marketplace_name(plugin_dir: &Path) -> Result<MarketplaceName> {
    for ancestor in bounded_ancestors(plugin_dir) {
        let path = ancestor.join(".claude-plugin/marketplace.json");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let manifest = |reason: String| Error::Manifest {
            path: path.display().to_string(),
            reason,
        };
        let value: Value = serde_json::from_str(&text)
            .map_err(|source| manifest(format!("not JSON: {source}")))?;
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| manifest(String::from("no `name` field")))?;
        return MarketplaceName::new(name).map_err(|e| manifest(e.to_string()));
    }
    // Unreachable in practice: the loop above returns on the first ancestor
    // holding a marketplace manifest, and `bounded_ancestors` stops at the
    // repository root, so a plugin in a repo that declares no marketplace
    // falls through to here rather than climbing into unrelated parents.
    Err(Error::Marketplace {
        path: plugin_dir
            .join("../.claude-plugin/marketplace.json")
            .display()
            .to_string(),
        reason: String::from(
            "no `.claude-plugin/marketplace.json` in any ancestor directory; \
             the installed layout is keyed by the marketplace that hosts the plugin",
        ),
    })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::{marketplace_name, read};

    /// A repo shaped like this one: marketplace at the root, plugin below it.
    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude-plugin")).unwrap();
        std::fs::write(
            dir.path().join(".claude-plugin/marketplace.json"),
            r#"{"name":"airsstack","plugins":[]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("plugins/demo/.claude-plugin")).unwrap();
        std::fs::write(
            dir.path().join("plugins/demo/.claude-plugin/plugin.json"),
            r#"{"name":"demo","version":"0.2.1"}"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn the_manifest_yields_the_name_and_version_the_cache_path_needs() {
        let dir = repo();
        let manifest = read(&dir.path().join("plugins/demo")).unwrap();
        assert_eq!(manifest.name.as_str(), "demo");
        assert_eq!(manifest.version.as_str(), "0.2.1");
    }

    #[test]
    fn the_marketplace_name_is_found_by_walking_up_from_the_plugin() {
        let dir = repo();
        let found = marketplace_name(&dir.path().join("plugins/demo")).unwrap();
        assert_eq!(found.as_str(), "airsstack");
    }

    #[test]
    fn a_plugin_with_no_manifest_is_an_error_naming_the_file_it_wanted() {
        let dir = tempfile::tempdir().unwrap();
        let error = read(dir.path()).unwrap_err().to_string();
        assert!(error.contains("plugin.json"), "{error}");
    }

    #[test]
    fn a_manifest_without_a_version_is_an_error_naming_the_field() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude-plugin")).unwrap();
        std::fs::write(
            dir.path().join(".claude-plugin/plugin.json"),
            r#"{"name":"demo"}"#,
        )
        .unwrap();
        let error = read(dir.path()).unwrap_err().to_string();
        assert!(error.contains("version"), "{error}");
    }

    #[test]
    fn the_search_stops_at_the_repository_root_rather_than_climbing_past_it() {
        // The arrangement this guards: a checkout nested inside another
        // directory that happens to hold a marketplace manifest. Without the
        // floor the walk reaches the outer one and keys the install layout by
        // a marketplace that does not host this plugin at all.
        let outer = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(outer.path().join(".claude-plugin")).unwrap();
        std::fs::write(
            outer.path().join(".claude-plugin/marketplace.json"),
            r#"{"name":"not-the-host","plugins":[]}"#,
        )
        .unwrap();

        let repo = outer.path().join("checkout");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        let plugin = repo.join("plugins/demo");
        std::fs::create_dir_all(plugin.join(".claude-plugin")).unwrap();
        std::fs::write(
            plugin.join(".claude-plugin/plugin.json"),
            r#"{"name":"demo","version":"0.1.0"}"#,
        )
        .unwrap();

        let error = marketplace_name(&plugin).unwrap_err().to_string();
        assert!(error.contains("marketplace.json"), "{error}");
        assert!(!error.contains("not-the-host"), "{error}");
    }

    #[test]
    fn a_marketplace_at_the_repository_root_is_still_found() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(repo.path().join(".claude-plugin")).unwrap();
        std::fs::write(
            repo.path().join(".claude-plugin/marketplace.json"),
            r#"{"name":"airsstack","plugins":[]}"#,
        )
        .unwrap();
        let plugin = repo.path().join("plugins/demo");
        std::fs::create_dir_all(&plugin).unwrap();

        // The floor is inclusive: the root that carries `.git` is searched
        // before the walk stops, which is where this repo keeps its manifest.
        let found = marketplace_name(&plugin).unwrap();
        assert_eq!(found.as_str(), "airsstack");
    }

    #[test]
    fn a_plugin_outside_any_marketplace_is_an_error_that_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let error = marketplace_name(dir.path()).unwrap_err().to_string();
        assert!(error.contains("marketplace.json"), "{error}");
    }
}
