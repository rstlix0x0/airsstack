//! Sequential runner over a suite of cases.

use std::future::poll_fn;

use crate::agent::client::Client;
use crate::agent::evals::case::Case;
use crate::agent::evals::error::EvalError;
use crate::agent::evals::outcome::Outcome;
use crate::agent::evals::report::{CaseReport, Report};
use crate::agent::runtime::Runtime;

/// A suite of eval cases run sequentially against a client.
#[derive(Default)]
pub struct EvalSuite {
    cases: Vec<Case>,
}

impl EvalSuite {
    /// An empty suite.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a case. Chainable.
    #[must_use]
    pub fn case(mut self, case: Case) -> Self {
        self.cases.push(case);
        self
    }

    /// The number of cases in the suite.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cases.len()
    }

    /// Whether the suite has no cases.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    /// Run every case against `client`, sequentially, returning the aggregate report.
    ///
    /// Generic over the runtime — the runtime-agnostic seam.
    ///
    /// # Errors
    /// Returns [`EvalError::Run`] if driving the subject runtime for any case
    /// fails; scorer/grader problems degrade to failing scores, not errors.
    pub async fn run<R: Runtime>(&self, client: &Client<R>) -> Result<Report, EvalError> {
        let mut reports = Vec::with_capacity(self.cases.len());
        for case in &self.cases {
            let mut stream = client
                .query(case.prompt().clone())
                .await
                .map_err(|source| EvalError::Run {
                    case: case.name().to_string(),
                    source,
                })?;
            let mut messages = Vec::new();
            while let Some(item) = poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
                match item {
                    Ok(message) => messages.push(message),
                    Err(source) => {
                        return Err(EvalError::Run {
                            case: case.name().to_string(),
                            source,
                        });
                    }
                }
            }
            let outcome = Outcome::from_messages(messages);
            let mut scores = Vec::with_capacity(case.scorers().len());
            for scorer in case.scorers() {
                scores.push((scorer.label().to_string(), scorer.score(&outcome).await));
            }
            let passed = scores.iter().all(|(_, score)| score.passed);
            reports.push(CaseReport {
                name: case.name().to_string(),
                scores,
                passed,
            });
        }
        Ok(Report::new(reports))
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::EvalSuite;
    use crate::agent::client::Client;
    use crate::agent::content::ContentBlock;
    use crate::agent::evals::case::Case;
    use crate::agent::evals::scorers::{contains, no_error};
    use crate::agent::message::{AssistantMessage, Message, ResultMessage};
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::types::SessionId;

    fn assistant(text: &str) -> Message {
        Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::Text { text: text.into() }],
            parent_tool_use_id: None,
        })
    }

    fn result(is_error: bool) -> Message {
        Message::Result(ResultMessage {
            result: String::new(),
            is_error,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })
    }

    #[tokio::test]
    async fn runs_cases_and_aggregates_pass_fail() {
        // Turn 1: text "hello" + clean result -> both scorers pass.
        // Turn 2: text "bye" + error result -> contains("hello") and no_error fail.
        let client = Client::with_runtime(MockRuntime::new(vec![
            vec![assistant("hello"), result(false)],
            vec![assistant("bye"), result(true)],
        ]));
        let suite = EvalSuite::new()
            .case(
                Case::new("first", "hi")
                    .scorer(contains("hello"))
                    .scorer(no_error()),
            )
            .case(
                Case::new("second", "hi")
                    .scorer(contains("hello"))
                    .scorer(no_error()),
            );

        let report = suite.run(&client).await.expect("run");
        assert_eq!(report.total(), 2);
        assert_eq!(report.pass_count(), 1);
        assert!(!report.passed());
        assert!(report.cases()[0].passed);
        assert!(!report.cases()[1].passed);
    }

    #[tokio::test]
    async fn empty_scorer_set_passes_vacuously() {
        let client = Client::with_runtime(MockRuntime::new(vec![vec![result(false)]]));
        let suite = EvalSuite::new().case(Case::new("bare", "hi"));
        let report = suite.run(&client).await.expect("run");
        assert!(report.passed());
    }
}
