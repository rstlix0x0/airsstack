//! Built-in deterministic scorers. Each is infallible — it always yields a
//! [`Score`], never an error — and pulls in no extra dependency.

use async_trait::async_trait;

use crate::agent::evals::outcome::Outcome;
use crate::agent::evals::score::{Score, Scorer};

/// Passes when the assistant text contains a substring.
pub struct Contains {
    needle: String,
}

/// Passes when the assistant text equals a string exactly.
pub struct Equals {
    expected: String,
}

/// Passes when the outcome's result frame did not report an error.
pub struct NoError;

/// Passes when a tool of the given name was requested.
pub struct ToolCalled {
    name: String,
}

/// Passes when total (input + output) token usage is within a budget.
pub struct TokenBudget {
    max: u64,
}

/// Passes when a caller-supplied predicate over the outcome returns true.
pub struct Predicate<F> {
    label: String,
    test: F,
}

/// Assert the assistant text contains `needle`.
#[must_use]
pub fn contains(needle: impl Into<String>) -> Contains {
    Contains {
        needle: needle.into(),
    }
}

/// Assert the assistant text equals `expected`.
#[must_use]
pub fn equals(expected: impl Into<String>) -> Equals {
    Equals {
        expected: expected.into(),
    }
}

/// Assert the turn did not end in error.
#[must_use]
pub const fn no_error() -> NoError {
    NoError
}

/// Assert a tool named `name` was requested.
#[must_use]
pub fn tool_called(name: impl Into<String>) -> ToolCalled {
    ToolCalled { name: name.into() }
}

/// Assert total token usage is at most `max` (fails when no usage is reported).
#[must_use]
pub const fn token_budget(max: u64) -> TokenBudget {
    TokenBudget { max }
}

/// Assert a caller-defined predicate holds; the escape hatch for bespoke checks.
pub fn predicate<F>(label: impl Into<String>, test: F) -> Predicate<F>
where
    F: Fn(&Outcome) -> bool + Send + Sync + 'static,
{
    Predicate {
        label: label.into(),
        test,
    }
}

#[async_trait]
impl Scorer for Contains {
    fn label(&self) -> &'static str {
        "contains"
    }
    async fn score(&self, outcome: &Outcome) -> Score {
        Score::boolean(outcome.assistant_text().contains(&self.needle))
    }
}

#[async_trait]
impl Scorer for Equals {
    fn label(&self) -> &'static str {
        "equals"
    }
    async fn score(&self, outcome: &Outcome) -> Score {
        Score::boolean(outcome.assistant_text() == self.expected)
    }
}

#[async_trait]
impl Scorer for NoError {
    fn label(&self) -> &'static str {
        "no_error"
    }
    async fn score(&self, outcome: &Outcome) -> Score {
        Score::boolean(!outcome.is_error())
    }
}

#[async_trait]
impl Scorer for ToolCalled {
    fn label(&self) -> &'static str {
        "tool_called"
    }
    async fn score(&self, outcome: &Outcome) -> Score {
        Score::boolean(outcome.tool_uses().any(|name| name == self.name))
    }
}

#[async_trait]
impl Scorer for TokenBudget {
    fn label(&self) -> &'static str {
        "token_budget"
    }
    async fn score(&self, outcome: &Outcome) -> Score {
        outcome.usage().map_or_else(
            || Score::boolean(false),
            |usage| {
                Score::boolean(usage.input_tokens.saturating_add(usage.output_tokens) <= self.max)
            },
        )
    }
}

#[async_trait]
impl<F> Scorer for Predicate<F>
where
    F: Fn(&Outcome) -> bool + Send + Sync,
{
    fn label(&self) -> &str {
        &self.label
    }
    async fn score(&self, outcome: &Outcome) -> Score {
        Score::boolean((self.test)(outcome))
    }
}

#[cfg(test)]
mod tests {
    use super::{contains, equals, no_error, predicate, token_budget, tool_called};
    use crate::agent::content::ContentBlock;
    use crate::agent::evals::outcome::Outcome;
    use crate::agent::evals::score::Scorer;
    use crate::agent::message::{AssistantMessage, Message, ResultMessage, Usage};
    use crate::agent::types::SessionId;

    fn text_outcome(text: &str) -> Outcome {
        Outcome::from_messages(vec![Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::Text { text: text.into() }],
            parent_tool_use_id: None,
        })])
    }

    fn tool_outcome(name: &str) -> Outcome {
        Outcome::from_messages(vec![Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::ToolUse {
                id: "1".into(),
                name: name.into(),
                input: serde_json::Value::Null,
            }],
            parent_tool_use_id: None,
        })])
    }

    fn result_outcome(is_error: bool, usage: Option<Usage>) -> Outcome {
        Outcome::from_messages(vec![Message::Result(ResultMessage {
            result: String::new(),
            is_error,
            total_cost_usd: None,
            stop_reason: None,
            usage,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })])
    }

    #[tokio::test]
    async fn contains_pass_and_fail() {
        assert!(contains("ell").score(&text_outcome("hello")).await.passed);
        assert!(!contains("zzz").score(&text_outcome("hello")).await.passed);
    }

    #[tokio::test]
    async fn equals_pass_and_fail() {
        assert!(equals("hello").score(&text_outcome("hello")).await.passed);
        assert!(!equals("hell").score(&text_outcome("hello")).await.passed);
    }

    #[tokio::test]
    async fn no_error_reads_result_frame() {
        assert!(no_error().score(&result_outcome(false, None)).await.passed);
        assert!(!no_error().score(&result_outcome(true, None)).await.passed);
    }

    #[tokio::test]
    async fn tool_called_matches_name() {
        assert!(
            tool_called("search")
                .score(&tool_outcome("search"))
                .await
                .passed
        );
        assert!(
            !tool_called("search")
                .score(&tool_outcome("write"))
                .await
                .passed
        );
    }

    #[tokio::test]
    async fn token_budget_pass_fail_and_missing_usage() {
        let within = result_outcome(
            false,
            Some(Usage {
                input_tokens: 3,
                output_tokens: 2,
                ..Default::default()
            }),
        );
        let over = result_outcome(
            false,
            Some(Usage {
                input_tokens: 30,
                output_tokens: 20,
                ..Default::default()
            }),
        );
        assert!(token_budget(10).score(&within).await.passed);
        assert!(!token_budget(10).score(&over).await.passed);
        assert!(
            !token_budget(10)
                .score(&result_outcome(false, None))
                .await
                .passed
        );
    }

    #[tokio::test]
    async fn predicate_runs_the_closure() {
        let scorer = predicate("has_tool", |o| o.tool_uses().count() == 1);
        assert_eq!(scorer.label(), "has_tool");
        assert!(scorer.score(&tool_outcome("search")).await.passed);
        assert!(!scorer.score(&text_outcome("x")).await.passed);
    }
}
