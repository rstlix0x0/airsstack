//! `claudevs doctor`: what this environment can and cannot do.
//!
//! Every probe here answers for one thing a `check` stage needs: whether the
//! `claude` binary is on `PATH` for the validate stage, whether the plugin's
//! own manifest can be read, whether any case files exist for the suite
//! stages to run at all, whether a marketplace can be found above the plugin
//! to key an install path by, and whether the simulated install layout can be
//! materialized. A gap is reported, never repaired, and never guessed at: the
//! probe runs the same code the stage would.
//!
//! [`ProbeStatus`] has two failing shades, chosen by what the probe is
//! actually reporting on:
//!
//! - **gap** — the *environment* around the plugin is missing something the
//!   plugin cannot supply for itself: no `claude` binary, no marketplace
//!   manifest above it, no simulated install layout, or (despite being
//!   content the plugin owns) no readable plugin manifest — without it
//!   neither the marketplace nor the install-layout probe can even run, so a
//!   missing manifest is a placement problem for those probes, not merely
//!   advisory.
//! - **warn** — an *observation* about the plugin's own content that does
//!   not, by itself, stop `check` from gating. Today that is only the cases
//!   probe: a plugin with no case files yet is incomplete, not broken, and
//!   `Diagnosis::all_clear` must not fail a plugin for it.
//!
//! Responsibilities: [`ProbeStatus`], [`Probe`], [`Diagnosis`], [`run`].

use std::path::Path;

use crate::layout::{Installed, marketplace_name, read_manifest};
use crate::validate::Validation;

/// How one probe came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeStatus {
    /// What was probed is usable.
    Ok,
    /// An observation about the plugin's own content; does not fail
    /// [`Diagnosis::all_clear`].
    Warning,
    /// The environment is missing something; fails [`Diagnosis::all_clear`].
    Gap,
}

/// One thing that was looked at.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Probe {
    /// What was probed.
    pub name: &'static str,
    /// How it came out.
    pub status: ProbeStatus,
    /// What was found, or what is missing.
    pub detail: String,
}

/// Every probe, in order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Diagnosis {
    /// The probes.
    pub probes: Vec<Probe>,
}

impl Diagnosis {
    /// Whether no probe reports a gap.
    ///
    /// A [`ProbeStatus::Warning`] does not count: it is an observation about
    /// the plugin's own content, not a reason to fail the environment it runs
    /// in.
    #[must_use]
    pub fn all_clear(&self) -> bool {
        self.probes
            .iter()
            .all(|probe| probe.status != ProbeStatus::Gap)
    }
}

/// Diagnoses the environment around the plugin at `plugin_dir`.
#[must_use]
pub fn run(plugin_dir: &Path) -> Diagnosis {
    run_with(plugin_dir, crate::validate::run)
}

/// The diagnosis, with the validation delegate supplied by the caller.
///
/// The delegate is a parameter for the same reason `check::run_with`'s is: it
/// is the one probe that leaves the machine, so pinning it is what keeps a
/// test asserting doctor's own behaviour instead of the installed `claude`
/// binary's. A release that tightens `--strict` would otherwise turn the
/// binary probe red under a test that has nothing to do with that release.
fn run_with(plugin_dir: &Path, validate: impl Fn(&Path) -> Validation) -> Diagnosis {
    Diagnosis {
        probes: vec![
            binary_probe(validate(plugin_dir)),
            manifest_probe(plugin_dir),
            cases_probe(plugin_dir),
            marketplace_probe(plugin_dir),
            layout_probe(plugin_dir),
        ],
    }
}

/// Whether the `claude` binary can be delegated to.
fn binary_probe(outcome: Validation) -> Probe {
    match outcome {
        Validation::Unavailable { reason } => Probe {
            name: "claude binary",
            status: ProbeStatus::Gap,
            detail: format!("{reason}; `check` skips its validate stage"),
        },
        Validation::Passed { .. } | Validation::Failed { .. } => Probe {
            name: "claude binary",
            status: ProbeStatus::Ok,
            detail: String::from("present; `check` delegates its validate stage"),
        },
    }
}

/// Whether the plugin's own manifest can be read.
fn manifest_probe(plugin_dir: &Path) -> Probe {
    match read_manifest(plugin_dir) {
        Ok(manifest) => Probe {
            name: "plugin manifest",
            status: ProbeStatus::Ok,
            detail: format!("{} {}", manifest.name, manifest.version),
        },
        Err(error) => Probe {
            name: "plugin manifest",
            status: ProbeStatus::Gap,
            detail: error.to_string(),
        },
    }
}

/// Whether any case files can be discovered for the suite stages to run.
///
/// A plugin with no cases yet is incomplete, not broken: this probe reports
/// [`ProbeStatus::Warning`] rather than [`ProbeStatus::Gap`], so it never
/// fails [`Diagnosis::all_clear`] on its own.
fn cases_probe(plugin_dir: &Path) -> Probe {
    match crate::case::discover(plugin_dir) {
        Ok(cases) => Probe {
            name: "cases",
            status: ProbeStatus::Ok,
            detail: format!("{} case file(s) found", cases.len()),
        },
        Err(error) => Probe {
            name: "cases",
            status: ProbeStatus::Warning,
            detail: error.to_string(),
        },
    }
}

/// Whether a marketplace can be found above the plugin.
fn marketplace_probe(plugin_dir: &Path) -> Probe {
    match marketplace_name(plugin_dir) {
        Ok(name) => Probe {
            name: "marketplace",
            status: ProbeStatus::Ok,
            detail: name.to_string(),
        },
        Err(error) => Probe {
            name: "marketplace",
            status: ProbeStatus::Gap,
            detail: error.to_string(),
        },
    }
}

/// Whether the simulated install layout can be materialized.
fn layout_probe(plugin_dir: &Path) -> Probe {
    match Installed::materialize(plugin_dir) {
        Ok(installed) => Probe {
            name: "install layout",
            status: ProbeStatus::Ok,
            detail: format!("simulated at {}", installed.plugin_root().display()),
        },
        Err(error) => Probe {
            name: "install layout",
            status: ProbeStatus::Gap,
            detail: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use std::path::PathBuf;

    use super::{ProbeStatus, run_with};
    use crate::validate::Validation;

    /// A delegate that is present, so a probe assertion is about doctor rather
    /// than about the `claude` binary installed on this machine.
    fn delegate_present(_: &std::path::Path) -> Validation {
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
    fn every_probe_is_reported_for_the_exemplar() {
        let diagnosis = run_with(&fixture("minimal-plugin"), delegate_present);
        let names: Vec<&str> = diagnosis.probes.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            vec![
                "claude binary",
                "plugin manifest",
                "cases",
                "marketplace",
                "install layout"
            ],
            "{diagnosis:?}"
        );
    }

    #[test]
    fn the_exemplars_manifest_marketplace_and_layout_are_all_present() {
        let diagnosis = run_with(&fixture("minimal-plugin"), delegate_present);
        for probe in diagnosis
            .probes
            .iter()
            .filter(|p| p.name != "claude binary")
        {
            assert_eq!(probe.status, ProbeStatus::Ok, "{probe:?}");
        }
        assert!(diagnosis.all_clear(), "{diagnosis:?}");

        // Pinned to the fixture corpus's own marketplace name, not just `ok`,
        // so this proves the probe read `tests/fixtures/.claude-plugin/
        // marketplace.json` rather than climbing past it to some ancestor's.
        let marketplace = diagnosis
            .probes
            .iter()
            .find(|p| p.name == "marketplace")
            .unwrap();
        assert_eq!(marketplace.detail, "claudevs-fixtures", "{marketplace:?}");
    }

    #[test]
    fn an_absent_binary_is_a_gap_that_names_the_skipped_stage() {
        let diagnosis = run_with(&fixture("minimal-plugin"), delegate_absent);
        let binary = diagnosis
            .probes
            .iter()
            .find(|p| p.name == "claude binary")
            .unwrap();
        assert_eq!(binary.status, ProbeStatus::Gap, "{binary:?}");
        assert!(binary.detail.contains("validate"), "{binary:?}");
        assert!(!diagnosis.all_clear(), "{diagnosis:?}");
    }

    #[test]
    fn the_exemplars_cases_probe_finds_a_non_zero_count() {
        let diagnosis = run_with(&fixture("minimal-plugin"), delegate_present);
        let cases = diagnosis.probes.iter().find(|p| p.name == "cases").unwrap();
        assert_eq!(cases.status, ProbeStatus::Ok, "{cases:?}");
        assert!(cases.detail.contains("case file"), "{cases:?}");
        assert!(!cases.detail.starts_with('0'), "{cases:?}");
    }

    /// A plugin with a manifest and a marketplace above it, but no
    /// `tests/`: every probe but `cases` finds what it needs, so this proves
    /// the warning tier in isolation rather than alongside an unrelated gap.
    fn plugin_with_no_cases() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".claude-plugin")).unwrap();
        std::fs::write(
            root.path().join(".claude-plugin/marketplace.json"),
            r#"{"name":"doctor-fixtures"}"#,
        )
        .unwrap();
        let plugin = root.path().join("plugin");
        std::fs::create_dir_all(plugin.join(".claude-plugin")).unwrap();
        std::fs::write(
            plugin.join(".claude-plugin/plugin.json"),
            r#"{"name":"no-cases-plugin","version":"0.1.0"}"#,
        )
        .unwrap();
        root
    }

    #[test]
    fn a_plugin_with_no_case_files_is_a_warning_not_a_gap() {
        let root = plugin_with_no_cases();
        let diagnosis = run_with(&root.path().join("plugin"), delegate_present);
        let cases = diagnosis.probes.iter().find(|p| p.name == "cases").unwrap();
        assert_eq!(cases.status, ProbeStatus::Warning, "{cases:?}");
        assert!(cases.detail.contains("no case files found"), "{cases:?}");
    }

    #[test]
    fn the_cases_warning_does_not_make_all_clear_false() {
        let root = plugin_with_no_cases();
        let diagnosis = run_with(&root.path().join("plugin"), delegate_present);
        for probe in diagnosis
            .probes
            .iter()
            .filter(|p| p.name != "claude binary" && p.name != "cases")
        {
            assert_eq!(probe.status, ProbeStatus::Ok, "{probe:?}");
        }
        assert!(
            diagnosis.all_clear(),
            "a cases warning alone must not fail all_clear: {diagnosis:?}"
        );
    }

    #[test]
    fn a_plugin_with_no_manifest_reports_that_gap_and_is_not_clear() {
        let dir = tempfile::tempdir().unwrap();
        let diagnosis = run_with(dir.path(), delegate_present);
        let manifest = diagnosis
            .probes
            .iter()
            .find(|p| p.name == "plugin manifest")
            .unwrap();
        assert_eq!(manifest.status, ProbeStatus::Gap, "{manifest:?}");
        assert!(manifest.detail.contains("plugin.json"), "{manifest:?}");
        assert!(!diagnosis.all_clear());
    }
}
