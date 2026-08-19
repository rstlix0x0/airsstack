//! Declared native suites: `<plugin>/claudevs.toml`.
//!
//! ```toml
//! [[native]]
//! run = "airsl test --policy confined ."
//! ```
//!
//! Each `run` is spawned as `sh -c` in the plugin directory; claudevs asserts
//! only the exit code. The combined output is captured, not streamed, and is
//! reported (not parsed) under a failing line by the human renderer.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::path::Path;

use crate::error::{Error, Result};
use crate::harness::{DEFAULT_TIMEOUT, run_shell};

/// One native suite's outcome.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NativeOutcome {
    /// The declared command.
    pub command: String,
    /// Its exit code.
    pub exit: i32,
    /// Its combined stdout+stderr, captured (not streamed) and never parsed.
    pub output: String,
}

/// The claudevs.toml shape.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    native: Vec<NativeEntry>,
}

/// One `[[native]]` entry.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeEntry {
    run: String,
}

/// Runs the suites the plugin declares (no file = no suites).
///
/// # Errors
///
/// [`Error::Native`] for a malformed file or an unspawnable command.
pub(crate) fn run_declared(plugin_dir: &Path) -> Result<Vec<NativeOutcome>> {
    let path = plugin_dir.join("claudevs.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let config: Config = toml::from_str(&text).map_err(|e| Error::Native {
        command: path.display().to_string(),
        reason: e.to_string(),
    })?;

    let mut outcomes = Vec::new();
    for entry in config.native {
        let captured = run_shell(
            &entry.run,
            plugin_dir,
            &std::collections::BTreeMap::new(),
            None,
            DEFAULT_TIMEOUT,
        )
        .map_err(|e| Error::Native {
            command: entry.run.clone(),
            reason: e.to_string(),
        })?;
        outcomes.push(NativeOutcome {
            command: entry.run,
            exit: captured.exit,
            output: format!("{}{}", captured.stdout, captured.stderr),
        });
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::run_declared;

    #[test]
    fn no_config_file_means_no_native_suites() {
        let dir = tempfile::tempdir().unwrap();
        assert!(run_declared(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn a_declared_suite_runs_and_reports_its_exit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("claudevs.toml"),
            "[[native]]\nrun = \"echo native-ok; exit 0\"\n\n[[native]]\nrun = \"exit 4\"\n",
        )
        .unwrap();
        let outcomes = run_declared(dir.path()).unwrap();
        assert_eq!(outcomes.len(), 2);
        assert_eq!(outcomes[0].exit, 0);
        assert!(outcomes[0].output.contains("native-ok"));
        assert_eq!(outcomes[1].exit, 4);
    }

    #[test]
    fn a_malformed_config_is_an_error_naming_the_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("claudevs.toml"), "[[native]]\nrn = \"x\"\n").unwrap();
        assert!(run_declared(dir.path()).is_err());
    }
}
