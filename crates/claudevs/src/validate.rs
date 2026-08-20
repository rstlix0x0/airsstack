//! Delegating manifest validation to the `claude` binary.
//!
//! Spec §2: anything `claude plugin` ships is delegated, never rebuilt. The
//! binary is not a requirement — CI runs no real binary at all — so its absence
//! degrades this stage to skipped-with-reason and `check` continues on its
//! deterministic stages.
//!
//! Responsibilities: [`Validation`], [`run`].

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::Error;
use crate::harness::{DEFAULT_TIMEOUT, run as spawn};

/// The delegate's verdict.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Validation {
    /// The delegate exited 0.
    Passed {
        /// What it printed.
        output: String,
    },
    /// The delegate exited non-zero.
    Failed {
        /// What it printed.
        output: String,
    },
    /// The delegate could not be run at all.
    Unavailable {
        /// Why — the message `doctor` repeats.
        reason: String,
    },
}

/// The binary the validation stage delegates to.
const PROGRAM: &str = "claude";

/// Runs `claude plugin validate --strict <plugin_dir>`.
#[must_use]
pub fn run(plugin_dir: &Path) -> Validation {
    run_program(PROGRAM, plugin_dir)
}

/// Runs `<program> plugin validate --strict <plugin_dir>`.
///
/// Taking the program as a parameter is what lets the tests drive all three
/// outcomes without depending on a `claude` binary being installed.
fn run_program(program: &str, plugin_dir: &Path) -> Validation {
    let argv = [
        String::from(program),
        String::from("plugin"),
        String::from("validate"),
        String::from("--strict"),
        plugin_dir.display().to_string(),
    ];
    match spawn(&argv, plugin_dir, &BTreeMap::new(), None, DEFAULT_TIMEOUT) {
        Ok(captured) => {
            let output = format!("{}{}", captured.stdout, captured.stderr);
            if captured.exit == 0 {
                Validation::Passed { output }
            } else {
                Validation::Failed { output }
            }
        }
        Err(Error::Io { source, .. }) => Validation::Unavailable {
            reason: format!("cannot run `{program}`: {source}"),
        },
        Err(other) => Validation::Unavailable {
            reason: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]
    #![expect(
        clippy::panic,
        reason = "let-else diagnostics in tests panic by design"
    )]

    use super::{Validation, run_program};

    #[test]
    fn a_missing_binary_is_unavailable_with_a_reason_not_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = run_program("claude-that-is-not-installed", dir.path());
        let Validation::Unavailable { reason } = outcome else {
            panic!("expected Unavailable, got {outcome:?}");
        };
        assert!(reason.contains("claude-that-is-not-installed"), "{reason}");
    }

    #[test]
    fn a_zero_exit_from_the_delegate_passes_the_stage() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            run_program("true", dir.path()),
            Validation::Passed { .. }
        ));
    }

    #[test]
    fn a_non_zero_exit_from_the_delegate_fails_the_stage() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            run_program("false", dir.path()),
            Validation::Failed { .. }
        ));
    }
}
