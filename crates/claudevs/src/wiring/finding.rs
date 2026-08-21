//! What the static checkers report.
//!
//! Responsibilities: [`Severity`], [`Finding`], [`WiringReport`]. A finding is
//! a *result*, never an [`crate::Error`]: the checkers error only when they
//! cannot read the plugin at all, exactly as a failing case is a verdict
//! rather than an error.

/// How serious a wiring observation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Fails the wiring stage.
    Error,
    /// Reported, and does not fail the stage.
    Warning,
}

/// One thing a checker found.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Finding {
    /// Whether this fails the stage.
    pub severity: Severity,
    /// Which checker produced it: `refs`, `invocations` or `matchers`.
    pub checker: &'static str,
    /// The file it was found in, relative to the plugin root.
    pub file: String,
    /// 1-based line, when the checker knows one.
    pub line: Option<usize>,
    /// What is wrong.
    pub message: String,
}

/// Everything the checkers found, in checker order.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct WiringReport {
    /// The findings.
    pub findings: Vec<Finding>,
}

impl WiringReport {
    /// Whether nothing of [`Severity::Error`] was found.
    #[must_use]
    pub fn all_clear(&self) -> bool {
        self.findings
            .iter()
            .all(|finding| finding.severity == Severity::Warning)
    }

    /// How many errors and how many warnings, in that order.
    #[must_use]
    pub fn counts(&self) -> (usize, usize) {
        let errors = self
            .findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count();
        (errors, self.findings.len() - errors)
    }
}

#[cfg(test)]
mod tests {
    use super::{Finding, Severity, WiringReport};

    fn finding(severity: Severity) -> Finding {
        Finding {
            severity,
            checker: "refs",
            file: String::from("hooks/hooks.json"),
            line: Some(7),
            message: String::from("does not exist"),
        }
    }

    #[test]
    fn a_report_of_warnings_only_is_clear() {
        let report = WiringReport {
            findings: vec![finding(Severity::Warning)],
        };
        assert!(report.all_clear());
        assert_eq!(report.counts(), (0, 1));
    }

    #[test]
    fn one_error_finding_stops_the_report_being_clear() {
        let report = WiringReport {
            findings: vec![finding(Severity::Warning), finding(Severity::Error)],
        };
        assert!(!report.all_clear());
        assert_eq!(report.counts(), (1, 1));
    }

    #[test]
    fn an_empty_report_is_clear() {
        assert!(WiringReport::default().all_clear());
    }
}
