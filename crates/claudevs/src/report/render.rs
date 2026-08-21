//! Rendering a [`SuiteReport`], a [`crate::check::CheckReport`], or a
//! [`Diagnosis`] for people and for machines.
//!
//! Exit-code contract: 0 = green, 1 = verdict failures or findings, 2 is the
//! CLI's (claudevs could not run) and is never produced from a report.

use std::fmt::Write as _;

use crate::check::{CheckReport, StageStatus};
use crate::doctor::{Diagnosis, ProbeStatus};
use crate::error::{Error, Result};
use crate::harness::Verdict;
use crate::suite::SuiteReport;
use crate::wiring::{Severity, WiringReport};

/// Blocks anything outside this crate from implementing [`Report`].
mod sealed {
    pub trait Sealed {}

    impl Sealed for super::SuiteReport {}
    impl Sealed for super::CheckReport {}
    impl Sealed for super::WiringReport {}
    impl Sealed for super::Diagnosis {}
}

/// A report type [`render_json`] can serialize.
///
/// Sealed against a private marker trait: implemented only for the four
/// report types the crate emits ([`SuiteReport`], [`CheckReport`],
/// [`WiringReport`], [`Diagnosis`]), so the bound on `render_json` enforces
/// what its rustdoc says — "any report type" — rather than accepting
/// anything serializable.
pub trait Report: serde::Serialize + sealed::Sealed {}

impl Report for SuiteReport {}
impl Report for CheckReport {}
impl Report for WiringReport {}
impl Report for Diagnosis {}

/// The human rendering of a wiring report: one line per finding, then counts.
#[must_use]
pub fn render_wiring_human(report: &WiringReport) -> String {
    let mut out = String::new();
    for finding in &report.findings {
        let mark = match finding.severity {
            Severity::Error => "error",
            Severity::Warning => "warn ",
        };
        let location = finding.line.map_or_else(
            || finding.file.clone(),
            |line| format!("{}:{line}", finding.file),
        );
        let _ = writeln!(
            out,
            "  {mark}  {}  {location}  {}",
            finding.checker, finding.message
        );
    }
    let (errors, warnings) = report.counts();
    let _ = write!(
        out,
        "\n{errors} error{}, {warnings} warning{}\n",
        plural(errors),
        plural(warnings)
    );
    out
}

/// `s` unless the count is one.
const fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// The human rendering: one line per case, mismatches indented, a summary.
#[must_use]
pub fn render_human(report: &SuiteReport) -> String {
    let mut out = String::new();
    let mut failed = 0usize;

    for outcome in &report.outcomes {
        match &outcome.verdict {
            Verdict::Pass => {
                let _ = writeln!(out, "  ok    {}", outcome.name);
            }
            Verdict::Fail(mismatches) => {
                failed += 1;
                let _ = writeln!(out, "  FAIL  {}", outcome.name);
                for mismatch in mismatches {
                    let _ = writeln!(out, "        {mismatch}");
                }
            }
        }
    }
    for native in &report.native {
        let mark = if native.exit == 0 { "ok  " } else { "FAIL" };
        let _ = writeln!(
            out,
            "  {mark}  native: {} (exit {})",
            native.command, native.exit
        );
        if native.exit != 0 {
            for line in native.output.lines() {
                let _ = writeln!(out, "        {line}");
            }
        }
    }

    let native_failed = report.native.iter().filter(|n| n.exit != 0).count();
    let _ = write!(
        out,
        "\n{} passed, {} failed ({} cases, {} native suites)\n",
        report.outcomes.len() - failed,
        failed + native_failed,
        report.outcomes.len(),
        report.native.len(),
    );
    out
}

/// The machine rendering of any report type.
///
/// # Errors
///
/// [`Error::Render`] on a serialization failure; never in practice, since the
/// report types this crate emits serialize infallibly.
///
/// The [`Report`] bound is sealed to this crate's own report types, so an
/// arbitrary `Serialize` value is rejected at compile time rather than
/// silently accepted:
///
/// ```compile_fail
/// // "hello" is `Serialize` but not `claudevs::Report`, so this must not compile.
/// let _ = claudevs::render_json(&"hello");
/// ```
pub fn render_json<R: Report>(report: &R) -> Result<String> {
    serde_json::to_string_pretty(report).map_err(|source| Error::Render { source })
}

/// The process exit code a report maps to.
#[must_use]
pub fn exit_code(report: &SuiteReport) -> i32 {
    i32::from(!report.all_green())
}

/// The human rendering of a check report: one line per stage, detail indented.
#[must_use]
pub fn render_check_human(report: &CheckReport) -> String {
    let mut out = String::new();
    for stage in &report.stages {
        let mark = match stage.status {
            StageStatus::Passed => "ok  ",
            StageStatus::Failed => "FAIL",
            StageStatus::Skipped => "skip",
        };
        let _ = writeln!(out, "  {mark}  {}", stage.name);
        for line in stage.detail.lines().filter(|line| !line.trim().is_empty()) {
            let _ = writeln!(out, "        {line}");
        }
    }
    let skipped = report
        .stages
        .iter()
        .filter(|stage| stage.status == StageStatus::Skipped)
        .count();
    let failed = report
        .stages
        .iter()
        .filter(|stage| stage.status == StageStatus::Failed)
        .count();
    // `ran` excludes skipped stages: a stage that never executed was not
    // "run", and StageStatus exists precisely to make that distinction.
    let ran = report.stages.len() - skipped;
    let _ = write!(
        out,
        "\n{ran} stage{} run, {failed} failed, {skipped} skipped\n",
        plural(ran)
    );
    out
}

/// The process exit code a check report maps to.
#[must_use]
pub fn check_exit_code(report: &CheckReport) -> i32 {
    i32::from(!report.all_clear())
}

/// The human rendering of a diagnosis: one line per probe, then a verdict.
#[must_use]
pub fn render_doctor_human(diagnosis: &Diagnosis) -> String {
    let mut out = String::new();
    for probe in &diagnosis.probes {
        let mark = match probe.status {
            ProbeStatus::Ok => "ok  ",
            ProbeStatus::Warning => "warn",
            ProbeStatus::Gap => "gap ",
        };
        let _ = writeln!(out, "  {mark}  {}: {}", probe.name, probe.detail);
    }
    let gaps = diagnosis
        .probes
        .iter()
        .filter(|probe| probe.status == ProbeStatus::Gap)
        .count();
    let warnings = diagnosis
        .probes
        .iter()
        .filter(|probe| probe.status == ProbeStatus::Warning)
        .count();
    let _ = write!(
        out,
        "\n{gaps} gap{}, {warnings} warning{}\n",
        plural(gaps),
        plural(warnings)
    );
    out
}

/// The process exit code a diagnosis maps to.
#[must_use]
pub fn doctor_exit_code(diagnosis: &Diagnosis) -> i32 {
    i32::from(!diagnosis.all_clear())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::{
        check_exit_code, doctor_exit_code, exit_code, render_check_human, render_doctor_human,
        render_human, render_json, render_wiring_human,
    };
    use crate::doctor::{Diagnosis, Probe, ProbeStatus};
    use crate::harness::Verdict;
    use crate::native::NativeOutcome;
    use crate::suite::{CaseOutcome, SuiteReport};

    fn report() -> SuiteReport {
        SuiteReport {
            outcomes: vec![
                CaseOutcome {
                    name: String::from("good"),
                    verdict: Verdict::Pass,
                },
                CaseOutcome {
                    name: String::from("bad"),
                    verdict: Verdict::Fail(vec![String::from("exit: expected 0, got 2")]),
                },
            ],
            native: vec![NativeOutcome {
                command: String::from("echo x"),
                exit: 0,
                output: String::from("x\n"),
            }],
        }
    }

    #[test]
    fn the_human_rendering_lists_cases_and_summarizes() {
        let text = render_human(&report());
        assert!(text.contains("ok    good"));
        assert!(text.contains("FAIL  bad"));
        assert!(text.contains("exit: expected 0, got 2"));
        assert!(text.contains("1 passed, 1 failed"));
    }

    #[test]
    fn a_failing_native_suite_prints_its_captured_output() {
        // A failing delegated `airsl test` must not be silent; the human
        // rendering has to surface what the process actually printed.
        let report = SuiteReport {
            outcomes: vec![],
            native: vec![NativeOutcome {
                command: String::from("airsl test ."),
                exit: 1,
                output: String::from("1 of 244 assertions failed: fs.read denies /etc\n"),
            }],
        };
        let text = render_human(&report);
        assert!(text.contains("FAIL  native: airsl test ."));
        assert!(
            text.contains("1 of 244 assertions failed: fs.read denies /etc"),
            "{text}"
        );
    }

    #[test]
    fn a_passing_native_suite_stays_one_line() {
        let text = render_human(&report());
        let ok_line = text
            .lines()
            .find(|line| line.contains("native: echo x"))
            .unwrap();
        let ok_line_index = text.find(ok_line).unwrap() + ok_line.len();
        let next_line = text[ok_line_index..].lines().next().unwrap_or("");
        assert!(
            !next_line.starts_with("        "),
            "a passing native line must not be followed by its captured output: {next_line:?}"
        );
    }

    #[test]
    fn the_exit_code_is_one_on_any_failure_and_zero_when_green() {
        assert_eq!(exit_code(&report()), 1);
        let green = SuiteReport {
            outcomes: vec![],
            native: vec![],
        };
        assert_eq!(exit_code(&green), 0);
    }

    #[test]
    fn the_wiring_rendering_names_the_checker_the_file_and_the_line() {
        let report = crate::wiring::WiringReport {
            findings: vec![
                crate::wiring::Finding {
                    severity: crate::wiring::Severity::Error,
                    checker: "refs",
                    file: String::from("hooks/hooks.json"),
                    line: Some(7),
                    message: String::from("`${CLAUDE_PLUGIN_ROOT}/hooks/absent.sh` does not exist"),
                },
                crate::wiring::Finding {
                    severity: crate::wiring::Severity::Warning,
                    checker: "invocations",
                    file: String::from("hooks/orphan.sh"),
                    line: None,
                    message: String::from("`orphan.sh` is referenced by nothing in this plugin"),
                },
            ],
        };
        let text = render_wiring_human(&report);
        assert!(text.contains("error  refs  hooks/hooks.json:7"), "{text}");
        assert!(
            text.contains("warn   invocations  hooks/orphan.sh"),
            "{text}"
        );
        assert!(text.contains("1 error, 1 warning"), "{text}");
    }

    #[test]
    fn a_clean_wiring_report_says_so_rather_than_printing_nothing() {
        let text = render_wiring_human(&crate::wiring::WiringReport::default());
        assert!(text.contains("0 errors, 0 warnings"), "{text}");
    }

    #[test]
    fn the_json_rendering_round_trips_the_verdicts() {
        let json = render_json(&report()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["outcomes"][0]["name"], "good");
    }

    #[test]
    fn the_check_rendering_marks_each_stage_and_repeats_a_skip_reason() {
        let report = crate::check::CheckReport {
            stages: vec![
                crate::check::Stage {
                    name: "validate",
                    status: crate::check::StageStatus::Skipped,
                    detail: String::from(
                        "cannot run `claude`: No such file or directory (os error 2)",
                    ),
                },
                crate::check::Stage {
                    name: "wiring",
                    status: crate::check::StageStatus::Failed,
                    detail: String::from(
                        "error  refs  hooks/hooks.json:1  escapes the plugin root\n",
                    ),
                },
            ],
        };
        let text = render_check_human(&report);
        assert!(text.contains("skip  validate"), "{text}");
        assert!(text.contains("cannot run `claude`"), "{text}");
        assert!(text.contains("FAIL  wiring"), "{text}");
        assert!(text.contains("1 stage run, 1 failed, 1 skipped"), "{text}");
        assert_eq!(check_exit_code(&report), 1);
    }

    #[test]
    fn the_check_rendering_marks_a_passed_stage_and_does_not_count_a_skip_as_run() {
        let report = crate::check::CheckReport {
            stages: vec![
                crate::check::Stage {
                    name: "format",
                    status: crate::check::StageStatus::Passed,
                    detail: String::new(),
                },
                crate::check::Stage {
                    name: "validate",
                    status: crate::check::StageStatus::Skipped,
                    detail: String::from("no binary"),
                },
            ],
        };
        let text = render_check_human(&report);
        // Pinned exactly (not just `contains`) so a regression in the
        // two-space indent — which lines check up with `render_doctor_human`
        // and `render_human` — is actually caught.
        assert_eq!(text.lines().next(), Some("  ok    format"), "{text}");
        assert!(text.contains("1 stage run, 0 failed, 1 skipped"), "{text}");
        assert_eq!(check_exit_code(&report), 0);
    }

    #[test]
    fn a_check_whose_stages_all_passed_or_skipped_exits_zero() {
        let report = crate::check::CheckReport {
            stages: vec![crate::check::Stage {
                name: "validate",
                status: crate::check::StageStatus::Skipped,
                detail: String::from("no binary"),
            }],
        };
        assert_eq!(check_exit_code(&report), 0);
    }

    #[test]
    fn the_doctor_rendering_marks_each_probe_and_counts_the_gaps() {
        let diagnosis = Diagnosis {
            probes: vec![
                Probe {
                    name: "claude binary",
                    status: ProbeStatus::Ok,
                    detail: String::from("present; `check` delegates its validate stage"),
                },
                Probe {
                    name: "plugin manifest",
                    status: ProbeStatus::Gap,
                    detail: String::from(
                        "manifest `<plugin>/.claude-plugin/plugin.json`: no `name` field",
                    ),
                },
                // A second gap, so the count is asymmetric: with one of each,
                // a filter counting `Ok` instead of `Gap` still renders
                // "1 gap" and the assertion below proves nothing.
                Probe {
                    name: "marketplace",
                    status: ProbeStatus::Gap,
                    detail: String::from("no `.claude-plugin/marketplace.json` in any ancestor"),
                },
            ],
        };
        let text = render_doctor_human(&diagnosis);
        assert!(text.contains("ok    claude binary: present"), "{text}");
        assert!(
            text.contains("gap   plugin manifest: manifest `<plugin>/.claude-plugin/plugin.json`: no `name` field"),
            "{text}"
        );
        assert!(text.contains("2 gaps, 0 warnings\n"), "{text}");
        assert_eq!(doctor_exit_code(&diagnosis), 1);
    }

    #[test]
    fn the_doctor_rendering_marks_a_warning_and_does_not_count_it_as_a_gap() {
        let diagnosis = Diagnosis {
            probes: vec![
                Probe {
                    name: "claude binary",
                    status: ProbeStatus::Ok,
                    detail: String::from("present; `check` delegates its validate stage"),
                },
                Probe {
                    name: "cases",
                    status: ProbeStatus::Warning,
                    detail: String::from("no case files found under `<plugin>/tests`"),
                },
            ],
        };
        let text = render_doctor_human(&diagnosis);
        assert!(text.contains("warn  cases: no case files found"), "{text}");
        assert!(text.contains("0 gaps, 1 warning\n"), "{text}");
        assert_eq!(doctor_exit_code(&diagnosis), 0);
    }
}
