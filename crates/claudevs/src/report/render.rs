//! Rendering a [`SuiteReport`] for people and for machines.
//!
//! Exit-code contract (spec §3): 0 = green, 1 = verdict failures, 2 is the
//! CLI's (could-not-run) and never produced from a report.

use std::fmt::Write as _;

use crate::harness::Verdict;
use crate::suite::SuiteReport;

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

/// The machine rendering.
///
/// # Errors
///
/// Never in practice; the report types serialize infallibly.
pub fn render_json(report: &SuiteReport) -> serde_json::Result<String> {
    serde_json::to_string_pretty(report)
}

/// The process exit code a report maps to.
#[must_use]
pub fn exit_code(report: &SuiteReport) -> i32 {
    i32::from(!report.all_green())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::{exit_code, render_human, render_json};
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
    fn the_json_rendering_round_trips_the_verdicts() {
        let json = render_json(&report()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["outcomes"][0]["name"], "good");
    }
}
