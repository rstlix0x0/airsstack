//! Judging an [`Observed`] run against a case's [`Expectations`].
//!
//! Every mismatch is one precise sentence; a verdict lists all of them rather
//! than stopping at the first, so one run shows the full distance to green.

use std::path::Path;

use crate::case::Expectations;
use crate::harness::Observed;

/// The outcome of one case.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Verdict {
    /// Every expectation held.
    Pass,
    /// At least one expectation failed; each mismatch described.
    Fail(Vec<String>),
}

/// Judges `observed` (plus the project tree for file asserts) against `expect`.
#[must_use]
pub fn judge(expect: &Expectations, observed: &Observed, project: &Path) -> Verdict {
    let mut mismatches = Vec::new();

    // A killed child is never a pass: no expectation can vouch for a run that
    // did not finish on its own.
    if observed.timed_out {
        mismatches.push(String::from(
            "timeout: the child timed out and was killed before completing",
        ));
    }
    if let Some(exit) = expect.exit
        && observed.exit != exit
    {
        mismatches.push(format!("exit: expected {exit}, got {}", observed.exit));
    }
    if let Some(decision) = expect.decision
        && observed.decision != Some(decision)
    {
        mismatches.push(format!(
            "decision: expected {decision:?}, got {:?}",
            observed.decision
        ));
    }
    if expect.output.as_deref() == Some("none") && observed.emitted {
        mismatches.push(String::from("output: expected none, but the hook emitted"));
    }
    if let Some(needle) = &expect.context_contains
        && !observed
            .context
            .as_deref()
            .unwrap_or("")
            .contains(needle.as_str())
    {
        mismatches.push(format!(
            "context: expected to contain `{needle}`, got {:?}",
            observed.context
        ));
    }
    if let Some(needle) = &expect.stdout_contains
        && !observed.stdout.contains(needle.as_str())
    {
        mismatches.push(format!("stdout: expected to contain `{needle}`"));
    }
    if let Some(needle) = &expect.stderr_contains
        && !observed.stderr.contains(needle.as_str())
    {
        mismatches.push(format!("stderr: expected to contain `{needle}`"));
    }
    for rel in &expect.files_exist {
        if !project.join(rel).exists() {
            mismatches.push(format!("file: expected `{rel}` to exist in the project"));
        }
    }

    if mismatches.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Fail(mismatches)
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]
    #![expect(clippy::panic, reason = "tests panic to reject an unexpected shape")]

    use super::{Verdict, judge};
    use crate::case::{Decision, Expectations};
    use crate::harness::Observed;

    fn observed() -> Observed {
        Observed {
            exit: 0,
            decision: Some(Decision::Deny),
            context: Some(String::from("read the rust guideline")),
            emitted: true,
            timed_out: false,
            stdout: String::new(),
            stderr: String::from("blocked: lockfile"),
        }
    }

    #[test]
    fn all_matching_expectations_pass() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("audit.log"), "x").unwrap();
        let expect = Expectations {
            exit: Some(0),
            decision: Some(Decision::Deny),
            context_contains: Some(String::from("guideline")),
            stderr_contains: Some(String::from("lockfile")),
            files_exist: vec![String::from("audit.log")],
            ..Expectations::default()
        };
        assert_eq!(judge(&expect, &observed(), dir.path()), Verdict::Pass);
    }

    #[test]
    fn every_mismatch_is_reported_not_only_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let expect = Expectations {
            exit: Some(1),
            decision: Some(Decision::Allow),
            ..Expectations::default()
        };
        let Verdict::Fail(mismatches) = judge(&expect, &observed(), dir.path()) else {
            panic!("expected a failing verdict");
        };
        assert_eq!(mismatches.len(), 2);
    }

    #[test]
    fn output_none_fails_when_the_hook_emitted() {
        let dir = tempfile::tempdir().unwrap();
        let expect = Expectations {
            output: Some(String::from("none")),
            ..Expectations::default()
        };
        assert!(matches!(
            judge(&expect, &observed(), dir.path()),
            Verdict::Fail(_)
        ));
    }

    #[test]
    fn a_timed_out_run_never_passes() {
        let dir = tempfile::tempdir().unwrap();
        let mut timed_out = observed();
        timed_out.timed_out = true;
        let Verdict::Fail(mismatches) = judge(&Expectations::default(), &timed_out, dir.path())
        else {
            panic!("a killed run must not pass");
        };
        assert!(mismatches[0].contains("timed out"), "{mismatches:?}");
    }

    #[test]
    fn empty_expectations_pass_vacuously() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            judge(&Expectations::default(), &observed(), dir.path()),
            Verdict::Pass
        );
    }
}
