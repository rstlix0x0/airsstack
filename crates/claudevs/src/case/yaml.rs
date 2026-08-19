//! YAML front-end: one `.yaml` file → one [`Case`].
//!
//! The file body deserializes to `serde_json::Value` first, so YAML and the Lua
//! data front-end share the exact same [`RawCase`] path — the equivalence that
//! `migrate` depends on.

use std::path::Path;

use crate::case::{Case, RawCase};
use crate::error::{Error, Result};
use crate::types::CaseName;

/// Loads one YAML case file.
///
/// The case name is the file stem.
///
/// # Errors
///
/// Returns [`Error::CaseLoad`] for unreadable files, invalid YAML, unknown
/// fields, an invalid stem, or a shape [`Case::from_raw`] rejects.
pub fn load(path: &Path) -> Result<Case> {
    let fail = |reason: String| Error::CaseLoad {
        path: path.display().to_string(),
        reason,
    };

    let text = std::fs::read_to_string(path).map_err(|e| fail(e.to_string()))?;
    let value: serde_json::Value =
        serde_yaml_ng::from_str(&text).map_err(|e| fail(e.to_string()))?;
    let raw: RawCase = serde_json::from_value(value).map_err(|e| fail(e.to_string()))?;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| fail("file has no UTF-8 stem".into()))?;
    let name = CaseName::new(stem).map_err(|e| fail(e.to_string()))?;

    Case::from_raw(name, raw).map_err(fail)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::load;

    fn write(body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("blocks-lockfile.yaml"), body).unwrap();
        dir
    }

    #[test]
    fn a_yaml_case_loads_and_takes_its_name_from_the_stem() {
        let dir = write(
            "event: PreToolUse\npayload:\n  tool_input:\n    file_path: Cargo.lock\nexpect:\n  decision: deny\n",
        );
        let case = load(&dir.path().join("blocks-lockfile.yaml")).unwrap();
        assert_eq!(case.name.as_str(), "blocks-lockfile");
    }

    #[test]
    fn an_unknown_field_fails_the_load_with_the_field_named() {
        let dir = write("event: PreToolUse\nexpct: {}\n");
        let error = load(&dir.path().join("blocks-lockfile.yaml"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("expct"), "{error}");
    }

    #[test]
    fn invalid_yaml_reports_the_file_path() {
        let dir = write("event: [unclosed\n");
        let error = load(&dir.path().join("blocks-lockfile.yaml"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("blocks-lockfile.yaml"), "{error}");
    }
}
