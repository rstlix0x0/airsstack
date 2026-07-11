//! The model-judge permission port and its inputs.
//!
//! A `PermissionJudge` decides allow/deny for one tool call, given the call
//! itself plus the surrounding context in a `JudgeRequest`. [`JudgeRubric`]
//! is optional caller guidance appended to a judge's built-in rulebook.

use thiserror::Error;

/// Failure building a [`JudgeRubric`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RubricError {
    /// The rubric text was empty or whitespace-only.
    #[error("judge rubric must not be empty")]
    Empty,
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
}
