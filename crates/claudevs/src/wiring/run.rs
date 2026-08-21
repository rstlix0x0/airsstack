//! Orchestrates the three wiring checkers into one [`WiringReport`].
//!
//! Split out from [`super`] (the `mod.rs`-is-export-only rule) because it is
//! the one item in this module with a body: [`run`] is the caller's single
//! entry point into wiring, composing `refs`, `invocations` and `matchers`
//! in that order.
//!
//! Responsibilities: [`run`].

use std::path::Path;

use crate::error::{Error, Result};
use crate::wiring::{WiringReport, invocations, matchers, refs};

/// Runs every checker over the plugin at `plugin_dir`, in checker order.
///
/// # Errors
///
/// [`crate::Error::Io`] when `plugin_dir` is not a directory — nothing there
/// at all, or a path that names a plain file — or when the plugin directory
/// cannot otherwise be walked. That is "there is no plugin here to check",
/// which is different from a broken plugin: a plugin whose contents are
/// wrong is findings, never an error.
pub fn run(plugin_dir: &Path) -> Result<WiringReport> {
    std::fs::read_dir(plugin_dir).map_err(|source| Error::Io {
        operation: "walk plugin",
        path: plugin_dir.display().to_string(),
        source,
    })?;

    let mut findings = refs::check(plugin_dir)?;
    findings.extend(invocations::check(plugin_dir)?);
    findings.extend(matchers::check(plugin_dir)?);
    Ok(WiringReport { findings })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::run;
    use crate::error::Error;
    use crate::wiring::Severity;

    #[test]
    fn one_run_carries_findings_from_every_checker_in_checker_order() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        // refs: a reference to a file that is not there.
        std::fs::write(
            dir.path().join("hooks/hooks.json"),
            r#"{"hooks":{"Nope":[{"matcher":"Edit(","hooks":[{"type":"command","command":"sh \"${CLAUDE_PLUGIN_ROOT}/hooks/absent.sh\""}]}]}}"#,
        )
        .unwrap();
        // invocations: a script nothing names.
        std::fs::write(dir.path().join("hooks/orphan.sh"), "exit 0\n").unwrap();

        let report = run(dir.path()).unwrap();
        let checkers: Vec<&str> = report.findings.iter().map(|f| f.checker).collect();
        assert_eq!(
            checkers,
            vec!["refs", "invocations", "matchers", "matchers"],
            "{report:?}"
        );
        assert!(!report.all_clear());
        assert_eq!(report.counts(), (3, 1));
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.severity == Severity::Warning && f.file == "hooks/orphan.sh"),
            "{report:?}"
        );
    }

    #[test]
    fn a_plugin_with_nothing_wrong_reports_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::write(dir.path().join("hooks/gate.sh"), "exit 0\n").unwrap();
        std::fs::write(
            dir.path().join("hooks/hooks.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Edit|Write","hooks":[{"type":"command","command":"sh \"${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh\""}]}]}}"#,
        )
        .unwrap();
        let report = run(dir.path()).unwrap();
        assert!(report.findings.is_empty(), "{report:?}");
        assert!(report.all_clear());
    }

    #[test]
    fn a_plugin_dir_that_is_actually_a_file_is_an_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_plugin = dir.path().join("plugin.txt");
        std::fs::write(&not_a_plugin, "not a plugin\n").unwrap();

        let err = run(&not_a_plugin).unwrap_err();
        assert!(
            matches!(err, Error::Io { operation, .. } if operation == "walk plugin"),
            "{err:?}"
        );
    }

    #[test]
    fn an_empty_directory_is_not_an_error_it_is_clean_findings() {
        // An empty directory is a plausible "plugin scaffolded but nothing in
        // it yet": unlike a path that names a file, it is still a directory,
        // so it is not "no plugin here to check". `run` reports it the same
        // as any other plugin with nothing wrong: no findings.
        let dir = tempfile::tempdir().unwrap();
        let report = run(dir.path()).unwrap();
        assert!(report.findings.is_empty(), "{report:?}");
        assert!(report.all_clear());
    }
}
