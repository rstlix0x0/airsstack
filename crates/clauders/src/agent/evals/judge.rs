//! LLM-as-judge scorer over a caller-supplied grader.

use async_trait::async_trait;

use crate::agent::evals::error::EvalError;
use crate::agent::evals::outcome::Outcome;
use crate::agent::evals::score::{Score, Scorer};

/// Default pass threshold applied by [`Judge`] until overridden.
const DEFAULT_THRESHOLD: f64 = 0.5;

/// Grades a prompt with a model (or any backend), returning the raw reply.
#[async_trait]
pub trait Grader: Send + Sync {
    /// Grade `prompt` (rubric + subject output) and return the grader's reply.
    ///
    /// # Errors
    /// Returns [`EvalError::Grader`] when the backing grader fails.
    async fn grade(&self, prompt: String) -> Result<String, EvalError>;
}

/// A scorer that asks a [`Grader`] to grade the outcome against a rubric.
pub struct Judge<G: Grader> {
    grader: G,
    rubric: String,
    threshold: f64,
}

impl<G: Grader> Judge<G> {
    /// Build a judge over `grader` with an empty rubric and the default threshold.
    pub const fn new(grader: G) -> Self {
        Self {
            grader,
            rubric: String::new(),
            threshold: DEFAULT_THRESHOLD,
        }
    }

    /// Set the grading rubric (the instruction sent to the grader).
    #[must_use]
    pub fn rubric(mut self, rubric: impl Into<String>) -> Self {
        self.rubric = rubric.into();
        self
    }

    /// Set the pass threshold: `passed = value >= threshold`.
    #[must_use]
    pub const fn threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }
}

/// Parse a leading `f64` in `[0, 1]` from a grader reply.
fn parse_score(reply: &str) -> Option<f64> {
    let token = reply.split_whitespace().next()?;
    let value: f64 = token.parse().ok()?;
    (0.0..=1.0).contains(&value).then_some(value)
}

#[async_trait]
impl<G: Grader> Scorer for Judge<G> {
    fn label(&self) -> &'static str {
        "judge"
    }
    async fn score(&self, outcome: &Outcome) -> Score {
        let prompt = format!("{}\n\n{}", self.rubric, outcome.assistant_text());
        let Ok(reply) = self.grader.grade(prompt).await else {
            return Score::boolean(false);
        };
        parse_score(&reply).map_or_else(
            || Score::boolean(false),
            |value| Score::graded(value, self.threshold).unwrap_or(Score::boolean(false)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Grader, Judge};
    use crate::agent::content::ContentBlock;
    use crate::agent::evals::error::EvalError;
    use crate::agent::evals::outcome::Outcome;
    use crate::agent::evals::score::Scorer;
    use crate::agent::message::{AssistantMessage, Message};
    use async_trait::async_trait;

    struct Canned(Result<String, ()>);

    #[async_trait]
    impl Grader for Canned {
        async fn grade(&self, _prompt: String) -> Result<String, EvalError> {
            self.0
                .clone()
                .map_err(|()| EvalError::Grader("stub failure".into()))
        }
    }

    fn outcome() -> Outcome {
        Outcome::from_messages(vec![Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::Text {
                text: "answer".into(),
            }],
            parent_tool_use_id: None,
        })])
    }

    #[tokio::test]
    async fn parsed_score_above_threshold_passes() {
        let judge = Judge::new(Canned(Ok("0.9".into()))).threshold(0.8);
        let score = judge.score(&outcome()).await;
        assert!(score.passed);
        #[expect(
            clippy::float_cmp,
            reason = "score.value is parsed from the literal \"0.9\" with no intervening arithmetic, so exact equality is the intended check"
        )]
        {
            assert_eq!(score.value, 0.9);
        }
    }

    #[tokio::test]
    async fn parsed_score_below_threshold_fails() {
        let judge = Judge::new(Canned(Ok("0.2".into()))).threshold(0.8);
        assert!(!judge.score(&outcome()).await.passed);
    }

    #[tokio::test]
    async fn unparsable_reply_degrades_to_fail() {
        let judge = Judge::new(Canned(Ok("not a number".into())));
        assert!(!judge.score(&outcome()).await.passed);
    }

    #[tokio::test]
    async fn grader_error_degrades_to_fail() {
        let judge = Judge::new(Canned(Err(())));
        assert!(!judge.score(&outcome()).await.passed);
    }
}
