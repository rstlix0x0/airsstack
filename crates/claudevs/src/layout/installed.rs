//! A throwaway copy of a plugin in the shape it has once installed.
//!
//! The runtime installs a plugin at `cache/<marketplace>/<plugin>/<version>/`
//! and records it in `installed_plugins.json` under the key
//! `<plugin>@<marketplace>` — the shape read off this machine's own registry.
//! Reproducing both is what makes `test --installed` a second context rather
//! than a second suite: the same cases run again with `CLAUDE_PLUGIN_ROOT`
//! pointed at the copy, which is where a path that only works in the source
//! checkout comes apart.
//!
//! The registry is written for shape fidelity alone — nothing in the engine
//! reads it back, and no environment variable is invented here to point a child
//! at it. It exists so the simulated layout is the same shape on disk as a real
//! install, rather than a plugin directory that merely sits at a cache path.
//!
//! Responsibilities: [`Installed`].

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::harness::copy_tree;
use crate::layout::manifest;

/// A materialized install-cache layout (deleted on drop).
#[derive(Debug)]
pub struct Installed {
    /// Root of the throwaway tree; dropping it removes the copy.
    root: tempfile::TempDir,
    /// The copied plugin's directory inside that tree.
    plugin_root: PathBuf,
    /// The synthetic registry file.
    registry: PathBuf,
}

impl Installed {
    /// Copies `plugin_dir` into a fresh cache tree and writes the registry.
    ///
    /// # Errors
    ///
    /// [`Error::Manifest`] when the plugin or marketplace manifest cannot be
    /// read; [`Error::Layout`] when the tree cannot be created; [`Error::Io`] on
    /// copy failure.
    pub fn materialize(plugin_dir: &Path) -> Result<Self> {
        let plugin = manifest::read(plugin_dir)?;
        let marketplace = manifest::marketplace_name(plugin_dir)?;

        let root = tempfile::tempdir().map_err(|source| Error::Layout {
            reason: format!("cannot create the cache tree: {source}"),
        })?;
        let plugin_root = root
            .path()
            .join("cache")
            .join(marketplace.as_str())
            .join(plugin.name.as_str())
            .join(plugin.version.as_str());
        std::fs::create_dir_all(&plugin_root).map_err(|source| Error::Io {
            operation: "create cache directory",
            path: plugin_root.display().to_string(),
            source,
        })?;
        copy_tree(plugin_dir, &plugin_root)?;

        let registry = root.path().join("installed_plugins.json");
        let record = serde_json::json!({
            "version": 2,
            "plugins": {
                format!("{}@{}", plugin.name.as_str(), marketplace.as_str()): [
                    {
                        "scope": "user",
                        "installPath": plugin_root.display().to_string(),
                        "version": plugin.version.as_str(),
                    }
                ]
            }
        });
        std::fs::write(&registry, format!("{record:#}\n")).map_err(|source| Error::Io {
            operation: "write installed_plugins.json",
            path: registry.display().to_string(),
            source,
        })?;

        Ok(Self {
            root,
            plugin_root,
            registry,
        })
    }

    /// The copied plugin's directory — what `CLAUDE_PLUGIN_ROOT` becomes.
    #[must_use]
    pub fn plugin_root(&self) -> &Path {
        &self.plugin_root
    }

    /// The synthetic registry file.
    #[must_use]
    pub fn registry_path(&self) -> &Path {
        &self.registry
    }

    /// The root of the throwaway tree.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.root.path()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::Installed;

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude-plugin")).unwrap();
        std::fs::write(
            dir.path().join(".claude-plugin/marketplace.json"),
            r#"{"name":"airsstack","plugins":[]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("plugins/demo/.claude-plugin")).unwrap();
        std::fs::create_dir_all(dir.path().join("plugins/demo/hooks")).unwrap();
        std::fs::write(
            dir.path().join("plugins/demo/.claude-plugin/plugin.json"),
            r#"{"name":"demo","version":"0.2.1"}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("plugins/demo/hooks/gate.sh"), "exit 0\n").unwrap();
        dir
    }

    #[test]
    fn the_copy_sits_at_the_cache_path_the_runtime_uses() {
        let dir = repo();
        let installed = Installed::materialize(&dir.path().join("plugins/demo")).unwrap();
        let tail: Vec<String> = installed
            .plugin_root()
            .components()
            .rev()
            .take(4)
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        assert_eq!(tail, vec!["0.2.1", "demo", "airsstack", "cache"]);
        assert!(installed.plugin_root().join("hooks/gate.sh").is_file());
    }

    #[test]
    fn the_copy_is_not_the_source_tree() {
        let dir = repo();
        let source = dir.path().join("plugins/demo");
        let installed = Installed::materialize(&source).unwrap();
        std::fs::write(installed.plugin_root().join("hooks/gate.sh"), "exit 9\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(source.join("hooks/gate.sh")).unwrap(),
            "exit 0\n",
            "writing into the copy must never reach the plugin's checkout"
        );
    }

    #[test]
    fn the_synthetic_registry_keys_the_record_the_way_the_real_one_does() {
        let dir = repo();
        let installed = Installed::materialize(&dir.path().join("plugins/demo")).unwrap();
        let text = std::fs::read_to_string(installed.registry_path()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["version"], 2);
        let records = &value["plugins"]["demo@airsstack"];
        assert_eq!(records[0]["version"], "0.2.1");
        assert_eq!(
            records[0]["installPath"],
            installed.plugin_root().display().to_string()
        );
    }

    #[test]
    fn a_plugin_outside_a_marketplace_cannot_be_installed_and_says_why() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude-plugin")).unwrap();
        std::fs::write(
            dir.path().join(".claude-plugin/plugin.json"),
            r#"{"name":"demo","version":"0.1.0"}"#,
        )
        .unwrap();
        let error = Installed::materialize(dir.path()).unwrap_err().to_string();
        assert!(error.contains("marketplace.json"), "{error}");
    }
}
