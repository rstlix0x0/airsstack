//! The model-judge permission port and its inputs.
//!
//! A [`PermissionJudge`] decides allow/deny for one tool call, given the call
//! itself plus the surrounding context in a [`JudgeRequest`]. [`JudgeRubric`]
//! is optional caller guidance appended to a judge's built-in rulebook.

use async_trait::async_trait;
use thiserror::Error;

use crate::agent::error::AgentError;
use crate::agent::permissions::{PermissionContext, PermissionDecision};

/// Failure building a [`JudgeRubric`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RubricError {
    /// The rubric text was empty or whitespace-only.
    #[error("judge rubric must not be empty")]
    Empty,
}

/// Everything a [`PermissionJudge`] sees about one tool call under review.
pub struct JudgeRequest<'a> {
    /// Name of the tool the agent wants to run.
    pub tool: &'a str,
    /// The tool's proposed input.
    pub input: &'a serde_json::Value,
    /// The originating task the agent is pursuing, for task-fit judging.
    pub task: Option<&'a str>,
    /// The agent's own text immediately preceding this tool call.
    pub rationale: Option<&'a str>,
    /// The inbound permission-request context.
    pub ctx: &'a PermissionContext,
}

/// Decides allow/deny for a single tool call.
///
/// Consulted by the native permission gate when the session's mode is
/// [`crate::agent::permissions::PermissionMode::Auto`]. Returns the canonical
/// [`PermissionDecision`]; a model-backed judge returns `allow()` or
/// `deny(reason)`.
#[async_trait]
pub trait PermissionJudge: Send + Sync {
    /// Judge whether the reviewed tool call may run.
    ///
    /// # Errors
    /// Returns [`AgentError`] when the judge cannot reach a verdict (for
    /// example, its backing model transport fails); the runtime surfaces the
    /// error and aborts the turn.
    async fn judge(&self, req: &JudgeRequest<'_>) -> Result<PermissionDecision, AgentError>;
}

/// App-specific judging guidance, appended to a judge's built-in rulebook.
///
/// Non-empty by construction (parse-don't-validate): the surrounding
/// whitespace is trimmed and an empty result is rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeRubric(String);

impl JudgeRubric {
    /// Trim surrounding whitespace and validate the result is non-empty.
    ///
    /// # Errors
    /// [`RubricError::Empty`] if the text is empty after trimming.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::agent::permissions::{JudgeRubric, RubricError};
    ///
    /// let rubric = JudgeRubric::new("  never edit prod  ").unwrap();
    /// assert_eq!(rubric.as_str(), "never edit prod");
    ///
    /// assert_eq!(JudgeRubric::new("   "), Err(RubricError::Empty));
    /// ```
    pub fn new(text: impl Into<String>) -> Result<Self, RubricError> {
        let text = text.into();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(RubricError::Empty);
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the validated rubric text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{JudgeRubric, RubricError};

    #[test]
    fn rejects_empty_and_whitespace() {
        assert_eq!(JudgeRubric::new(""), Err(RubricError::Empty));
        assert_eq!(JudgeRubric::new("   "), Err(RubricError::Empty));
    }

    #[test]
    fn trims_and_stores_valid() {
        let r = JudgeRubric::new("  never edit prod  ").expect("valid");
        assert_eq!(r.as_str(), "never edit prod");
    }

    #[tokio::test]
    async fn a_judge_impl_returns_a_decision_for_a_request() {
        use super::{JudgeRequest, PermissionJudge};
        use crate::agent::error::AgentError;
        use crate::agent::permissions::{PermissionContext, PermissionDecision};

        struct AllowAll;

        #[async_trait::async_trait]
        impl PermissionJudge for AllowAll {
            async fn judge(
                &self,
                _req: &JudgeRequest<'_>,
            ) -> Result<PermissionDecision, AgentError> {
                Ok(PermissionDecision::allow())
            }
        }

        let input = serde_json::json!({ "cmd": "ls" });
        let ctx = PermissionContext::default();
        let req = JudgeRequest {
            tool: "Bash",
            input: &input,
            task: Some("list files"),
            rationale: Some("I'll list the directory"),
            ctx: &ctx,
        };
        let decision = AllowAll.judge(&req).await.expect("judge");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
    }
}
