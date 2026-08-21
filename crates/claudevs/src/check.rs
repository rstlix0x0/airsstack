//! `claudevs check`: the one gate a plugin passes or fails.
//!
//! Four stages in order — delegated manifest validation, static wiring, the case
//! suite, the case suite again from the simulated install layout — stopping at
//! the first failure, because a plugin whose wiring is broken has nothing to
//! learn from a suite run.
//!
//! Two things are deliberately *not* failures: a stage that cannot run for want
//! of an environment (`claude` absent, no marketplace to key an install path by,
//! no case files) is skipped with a reason, and a wiring warning is reported
//! without stopping anything.
//!
//! Responsibilities: [`StageStatus`], [`Stage`], [`CheckReport`], [`run`].

use std::path::Path;

use crate::error::{Error, Result};
use crate::report::{render_human, render_wiring_human};
use crate::suite::SuiteOptions;
use crate::validate::Validation;

/// How one stage ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    /// Nothing wrong.
    Passed,
    /// Something wrong; the pipeline stops here.
    Failed,
    /// Could not run in this environment; the pipeline continues.
    Skipped,
}

/// One stage's outcome.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Stage {
    /// The stage name, as it appears in the summary.
    pub name: &'static str,
    /// How it ended.
    pub status: StageStatus,
    /// What it produced, or why it was skipped.
    pub detail: String,
}

/// Every stage that ran, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct CheckReport {
    /// The stages.
    pub stages: Vec<Stage>,
}

impl CheckReport {
    /// Whether no stage failed.
    #[must_use]
    pub fn all_clear(&self) -> bool {
        self.stages
            .iter()
            .all(|stage| stage.status != StageStatus::Failed)
    }
}

/// Runs the check pipeline over the plugin at `plugin_dir`.
///
/// # Errors
///
/// Only conditions that stop claudevs itself: an unwalkable plugin directory, an
/// unloadable case file. A failing plugin is a report, not an error.
pub fn run(plugin_dir: &Path) -> Result<CheckReport> {
    run_with(plugin_dir, crate::validate::run)
}

/// The pipeline, with the validation delegate supplied by the caller.
///
/// The delegate is a parameter for the same reason [`crate::validate`] takes a
/// program name: it is the one stage that leaves the machine, so pinning it is
/// what keeps a test asserting claudevs' own behaviour instead of the installed
/// `claude` binary's. A release that tightens `--strict` would otherwise turn
/// the first stage red and truncate every pipeline under test.
///
/// # Errors
///
/// Same conditions as [`run`].
fn run_with(plugin_dir: &Path, validate: impl Fn(&Path) -> Validation) -> Result<CheckReport> {
    let mut report = CheckReport::default();

    report.stages.push(validation_stage(validate(plugin_dir)));
    if !report.all_clear() {
        return Ok(report);
    }

    let wiring = crate::wiring::run(plugin_dir)?;
    report.stages.push(Stage {
        name: "wiring",
        status: if wiring.all_clear() {
            StageStatus::Passed
        } else {
            StageStatus::Failed
        },
        detail: render_wiring_human(&wiring),
    });
    if !report.all_clear() {
        return Ok(report);
    }

    let options = SuiteOptions::default();
    report.stages.push(suite_stage(
        "test",
        crate::suite::run_suite(plugin_dir, &options),
    )?);
    if !report.all_clear() {
        return Ok(report);
    }

    report.stages.push(suite_stage(
        "test --installed",
        crate::suite::run_suite_installed(plugin_dir, &options),
    )?);
    Ok(report)
}

/// The delegated validation stage, from whatever the delegate reported.
fn validation_stage(outcome: Validation) -> Stage {
    match outcome {
        Validation::Passed { output } => Stage {
            name: "validate",
            status: StageStatus::Passed,
            detail: output,
        },
        Validation::Failed { output } => Stage {
            name: "validate",
            status: StageStatus::Failed,
            detail: output,
        },
        Validation::Unavailable { reason } => Stage {
            name: "validate",
            status: StageStatus::Skipped,
            detail: reason,
        },
    }
}

/// One suite stage, turning the environment-gap errors into a skip.
///
/// The three skippable conditions are properties of the *environment*, not of
/// the plugin: no case files yet, no marketplace above the plugin to key an
/// install path by, no writable temp dir. A malformed `plugin.json` is
/// deliberately not among them — that is a plugin defect, and skipping it would
/// let a plugin that cannot be installed pass the gate on any machine where the
/// `claude` delegate is absent and the validation stage already skipped.
///
/// # Errors
///
/// Propagates every error that is not an environment gap.
fn suite_stage(name: &'static str, outcome: Result<crate::suite::SuiteReport>) -> Result<Stage> {
    match outcome {
        Ok(suite) => Ok(Stage {
            name,
            status: if suite.all_green() {
                StageStatus::Passed
            } else {
                StageStatus::Failed
            },
            detail: render_human(&suite),
        }),
        Err(error @ (Error::NoCases { .. } | Error::Marketplace { .. } | Error::Layout { .. })) => {
            Ok(Stage {
                name,
                status: StageStatus::Skipped,
                detail: error.to_string(),
            })
        }
        Err(error @ Error::Manifest { .. }) => Ok(Stage {
            name,
            status: StageStatus::Failed,
            detail: error.to_string(),
        }),
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use std::path::PathBuf;

    use super::{StageStatus, run_with, suite_stage};
    use crate::error::Error;
    use crate::validate::Validation;

    /// A delegate that always passes, so a stage order assertion is about the
    /// pipeline rather than about the `claude` binary installed on this machine.
    fn delegate_passes(_: &std::path::Path) -> Validation {
        Validation::Passed {
            output: String::from("✔ Validation passed\n"),
        }
    }

    /// A delegate that is absent, as on CI.
    fn delegate_absent(_: &std::path::Path) -> Validation {
        Validation::Unavailable {
            reason: String::from("cannot run `claude`: No such file or directory (os error 2)"),
        }
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn wiring_failing_stops_the_pipeline_before_the_suite_runs() {
        let report = run_with(&fixture("escape-plugin"), delegate_passes).unwrap();
        let names: Vec<&str> = report.stages.iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["validate", "wiring"], "{report:?}");
        assert_eq!(report.stages[1].status, StageStatus::Failed);
        assert!(!report.all_clear());
    }

    #[test]
    fn a_dead_file_warning_does_not_stop_the_pipeline() {
        let report = run_with(&fixture("dead-script-plugin"), delegate_passes).unwrap();
        let wiring = report.stages.iter().find(|s| s.name == "wiring").unwrap();
        assert_eq!(wiring.status, StageStatus::Passed);
        assert!(
            report.stages.len() > 2,
            "the pipeline must continue past a warning: {report:?}"
        );
    }

    #[test]
    fn every_stage_runs_for_the_exemplar_and_the_check_is_clear() {
        let report = run_with(&fixture("minimal-plugin"), delegate_passes).unwrap();
        let names: Vec<&str> = report.stages.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            vec!["validate", "wiring", "test", "test --installed"],
            "{report:?}"
        );
        assert!(report.all_clear(), "{report:?}");

        // `all_clear()` is true for a skip, so the three deterministic stages
        // are pinned to Passed by name — otherwise this holds while proving
        // nothing about whether the installed run actually happened.
        for name in ["wiring", "test", "test --installed"] {
            let stage = report.stages.iter().find(|s| s.name == name).unwrap();
            assert_eq!(stage.status, StageStatus::Passed, "{name}: {report:?}");
        }
        // `validate` is the one stage allowed to skip: CI has no `claude`.
        let validate = report.stages.iter().find(|s| s.name == "validate").unwrap();
        assert_ne!(validate.status, StageStatus::Failed, "{report:?}");
    }

    #[test]
    fn a_stage_that_cannot_run_is_skipped_with_a_reason_and_does_not_fail_the_check() {
        // dead-script-plugin has no case files at all, so the suite stages
        // cannot produce verdicts; the plugin is still not *wrong*.
        //
        // Naming the `test` stage is load-bearing. The delegate is absent here,
        // as on CI, so `validate` skips too — asserting that *some* stage
        // skipped would hold with the whole environment-gap arm of
        // `suite_stage` deleted.
        let report = run_with(&fixture("dead-script-plugin"), delegate_absent).unwrap();
        let suite = report.stages.iter().find(|s| s.name == "test").unwrap();
        assert_eq!(suite.status, StageStatus::Skipped, "{report:?}");
        assert!(suite.detail.contains("no case files found"), "{report:?}");
    }

    #[test]
    fn a_broken_plugin_manifest_fails_its_stage_rather_than_skipping_it() {
        // The gap this closes: with `claude` absent the validation stage skips,
        // so a plugin whose `plugin.json` cannot be read would otherwise reach
        // `test --installed`, be recorded as skipped, and leave the check
        // "clear" on a plugin that cannot be installed at all.
        let stage = suite_stage(
            "test --installed",
            Err(Error::Manifest {
                path: String::from("<plugin>/.claude-plugin/plugin.json"),
                reason: String::from("no `version` field"),
            }),
        )
        .unwrap();
        assert_eq!(stage.status, StageStatus::Failed);
        assert!(stage.detail.contains("no `version` field"), "{stage:?}");
    }

    #[test]
    fn a_plugin_sitting_outside_any_marketplace_only_skips_the_stage() {
        // The other half of the split: placement is an environment gap, so it
        // must stay a skip even though it arrives from the same call.
        let stage = suite_stage(
            "test --installed",
            Err(Error::Marketplace {
                path: String::from("<plugin>/../.claude-plugin/marketplace.json"),
                reason: String::from("no `.claude-plugin/marketplace.json` in any ancestor"),
            }),
        )
        .unwrap();
        assert_eq!(stage.status, StageStatus::Skipped);
    }
}
