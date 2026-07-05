//! The scorer verdict type and the async scorer trait.

use async_trait::async_trait;

use crate::agent::evals::error::EvalError;
use crate::agent::evals::outcome::Outcome;

/// A scorer's verdict: a value in `[0, 1]` and whether it passed.
#[derive(Clone, Debug)]
pub struct Score {
    /// The numeric score, always within `[0, 1]`.
    pub value: f64,
    /// Whether this score counts as a pass.
    pub passed: bool,
}

impl Score {
    /// Grade `value` against `threshold`: `passed = value >= threshold`.
    ///
    /// # Errors
    /// Returns [`EvalError::Score`] if `value` is outside `[0, 1]`.
    pub fn graded(value: f64, threshold: f64) -> Result<Self, EvalError> {
        if !(0.0..=1.0).contains(&value) {
            return Err(EvalError::Score { value });
        }
        Ok(Self {
            value,
            passed: value >= threshold,
        })
    }

    /// A boolean verdict: `value` is `1.0` when `passed`, else `0.0`.
    #[must_use]
    pub const fn boolean(passed: bool) -> Self {
        Self {
            value: if passed { 1.0 } else { 0.0 },
            passed,
        }
    }
}

/// Judges a single-turn [`Outcome`], producing a [`Score`].
///
/// Async so an LLM-judge can await its grader; deterministic scorers return
/// without awaiting.
#[async_trait]
pub trait Scorer: Send + Sync {
    /// Names this scorer's row in the report.
    fn label(&self) -> &str;
    /// Judge the outcome.
    async fn score(&self, outcome: &Outcome) -> Score;
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{Score, Scorer};
    use crate::agent::evals::outcome::Outcome;
    use async_trait::async_trait;

    struct Always(bool);

    #[async_trait]
    impl Scorer for Always {
        fn label(&self) -> &'static str {
            "always"
        }
        async fn score(&self, _outcome: &Outcome) -> Score {
            Score::boolean(self.0)
        }
    }

    #[test]
    fn scorer_is_object_safe() {
        let boxed: Box<dyn Scorer> = Box::new(Always(true));
        assert_eq!(boxed.label(), "always");
    }

    #[test]
    fn graded_threshold_boundary_passes() {
        let s = Score::graded(0.8, 0.8).expect("in range");
        assert!(s.passed);
    }

    #[test]
    fn graded_below_threshold_fails() {
        assert!(!Score::graded(0.5, 0.8).expect("in range").passed);
    }

    #[test]
    fn graded_out_of_range_errors() {
        assert!(Score::graded(1.5, 0.8).is_err());
    }

    #[test]
    fn boolean_maps_to_extremes() {
        #[expect(
            clippy::float_cmp,
            reason = "Score::boolean maps to the exact constants 1.0/0.0, so exact equality is the intended check"
        )]
        {
            assert_eq!(Score::boolean(true).value, 1.0);
        }
        assert!(!Score::boolean(false).passed);
    }
}
