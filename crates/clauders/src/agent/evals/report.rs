//! Aggregate results of running an eval suite.

use std::fmt;

use crate::agent::evals::score::Score;

/// The scored result of one case.
pub struct CaseReport {
    /// The case name.
    pub name: String,
    /// Each scorer's label paired with its score, in run order.
    pub scores: Vec<(String, Score)>,
    /// Whether every scorer passed (vacuously true when there are no scorers).
    pub passed: bool,
}

/// The aggregate report across every case in a suite.
pub struct Report {
    cases: Vec<CaseReport>,
}

impl Report {
    /// Build a report from per-case results.
    pub(crate) const fn new(cases: Vec<CaseReport>) -> Self {
        Self { cases }
    }

    /// Whether every case passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.cases.iter().all(|case| case.passed)
    }

    /// The number of cases that passed.
    #[must_use]
    pub fn pass_count(&self) -> usize {
        self.cases.iter().filter(|case| case.passed).count()
    }

    /// The total number of cases.
    #[must_use]
    pub fn total(&self) -> usize {
        self.cases.len()
    }

    /// The per-case reports.
    #[must_use]
    pub fn cases(&self) -> &[CaseReport] {
        &self.cases
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}/{} passed", self.pass_count(), self.total())?;
        for case in &self.cases {
            let mark = if case.passed { "PASS" } else { "FAIL" };
            writeln!(f, "  [{mark}] {}", case.name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CaseReport, Report};
    use crate::agent::evals::score::Score;

    fn case(name: &str, passed: bool) -> CaseReport {
        CaseReport {
            name: name.into(),
            scores: vec![("x".into(), Score::boolean(passed))],
            passed,
        }
    }

    #[test]
    fn aggregate_counts_and_passed() {
        let report = Report::new(vec![case("a", true), case("b", false)]);
        assert_eq!(report.total(), 2);
        assert_eq!(report.pass_count(), 1);
        assert!(!report.passed());
    }

    #[test]
    fn all_pass_reports_passed() {
        let report = Report::new(vec![case("a", true), case("b", true)]);
        assert!(report.passed());
    }

    #[test]
    fn display_summarises_cases() {
        let report = Report::new(vec![case("a", true), case("b", false)]);
        let text = report.to_string();
        assert!(text.starts_with("1/2 passed"));
        assert!(text.contains("[PASS] a"));
        assert!(text.contains("[FAIL] b"));
    }
}
