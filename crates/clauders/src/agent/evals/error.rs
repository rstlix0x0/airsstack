//! Error type for the evals harness.

use crate::agent::error::AgentError;

/// A failure raised while running or scoring an eval suite.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    /// Driving the subject runtime for a case failed (query or streamed frame).
    #[error("running case '{case}': {source}")]
    Run {
        /// Name of the case whose run failed.
        case: String,
        /// The underlying agent error.
        source: AgentError,
    },
    /// A grader implementation returned an error.
    #[error("grader failed: {0}")]
    Grader(String),
    /// A score value was constructed outside the `[0, 1]` range.
    #[error("invalid score {value}: must be in [0,1]")]
    Score {
        /// The offending value.
        value: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::EvalError;

    #[test]
    fn grader_display() {
        assert_eq!(
            EvalError::Grader("boom".into()).to_string(),
            "grader failed: boom"
        );
    }

    #[test]
    fn score_display() {
        assert_eq!(
            EvalError::Score { value: 1.5 }.to_string(),
            "invalid score 1.5: must be in [0,1]"
        );
    }
}
