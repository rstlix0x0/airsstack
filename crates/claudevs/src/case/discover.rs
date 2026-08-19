//! Finding a plugin's case files.
//!
//! Walks `<plugin>/tests/**` in sorted order (reproducible failure order —
//! the airsl-cli lesson) and classifies files; `tests/fixtures/` is data, not
//! cases. Zero discoveries is an error: a broken naming convention must not
//! read as a green suite.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// A discovered case file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseFile {
    /// A single-case YAML file.
    Yaml(PathBuf),
    /// A Lua file returning a table of cases.
    Lua(PathBuf),
}

/// Every case file under `<plugin>/tests`, sorted.
///
/// # Errors
///
/// [`Error::Io`] when the walk fails; [`Error::NoCases`] when nothing matches.
pub fn discover(plugin_dir: &Path) -> Result<Vec<CaseFile>> {
    let root = plugin_dir.join("tests");
    let mut found = Vec::new();

    if root.is_dir() {
        for entry in walkdir::WalkDir::new(&root).sort_by_file_name() {
            let entry = entry.map_err(|e| Error::Io {
                operation: "walk",
                path: root.display().to_string(),
                source: e.into(),
            })?;
            let path = entry.path();
            if !entry.file_type().is_file() || under_fixtures(path, &root) {
                continue;
            }
            if let Some(file) = classify(path) {
                found.push(file);
            }
        }
    }

    if found.is_empty() {
        return Err(Error::NoCases {
            root: root.display().to_string(),
        });
    }
    Ok(found)
}

/// Whether `path` sits under `tests/fixtures/`.
fn under_fixtures(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|rel| rel.components().next())
        .is_some_and(|first| first.as_os_str() == "fixtures")
}

/// The case-file kind of `path`, if it is one.
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the case-file naming convention (`.yaml`, `_test.lua`, `test_*.lua`) is an exact-case \
              contract, not a filesystem lookup; case-insensitive matching would silently accept \
              files the convention rejects"
)]
fn classify(path: &Path) -> Option<CaseFile> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".yaml") || name.ends_with(".yml") {
        return Some(CaseFile::Yaml(path.to_path_buf()));
    }
    if name.ends_with(".lua") && (name.ends_with("_test.lua") || name.starts_with("test_")) {
        return Some(CaseFile::Lua(path.to_path_buf()));
    }
    None
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::{CaseFile, discover};

    fn plugin() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures/repo")).unwrap();
        dir
    }

    fn touch(dir: &tempfile::TempDir, rel: &str) {
        std::fs::write(dir.path().join(rel), "").unwrap();
    }

    #[test]
    fn yaml_and_lua_test_files_are_found_sorted_and_fixtures_are_not() {
        let dir = plugin();
        touch(&dir, "tests/z-case.yaml");
        touch(&dir, "tests/a_test.lua");
        touch(&dir, "tests/helper.lua"); // not a test-file name
        touch(&dir, "tests/fixtures/repo/x.yaml"); // fixture data
        let found = discover(dir.path()).unwrap();
        assert_eq!(found.len(), 2);
        assert!(matches!(found[0], CaseFile::Lua(_)));
        assert!(matches!(found[1], CaseFile::Yaml(_)));
    }

    #[test]
    fn zero_discoveries_is_a_failure_not_a_pass() {
        let dir = plugin();
        touch(&dir, "tests/notes.txt");
        assert!(matches!(
            discover(dir.path()),
            Err(crate::Error::NoCases { .. })
        ));
    }
}
