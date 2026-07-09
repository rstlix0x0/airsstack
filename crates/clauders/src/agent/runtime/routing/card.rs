//! The routing catalog entry: a model identity paired with its summary.

use crate::types::ModelId;

use super::error::RoutingError;

/// The maximum length of a [`RoutingSummary`], in characters.
const MAX_SUMMARY_LEN: usize = 512;

/// A short, human-authored description of what a model is best suited for,
/// shown to the classifier when it chooses a target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutingSummary(String);

impl RoutingSummary {
    /// Trim surrounding whitespace and validate the result.
    ///
    /// # Errors
    /// [`RoutingError::EmptySummary`] if empty after trimming;
    /// [`RoutingError::SummaryTooLong`] if longer than `MAX_SUMMARY_LEN` chars.
    pub fn new(text: impl Into<String>) -> Result<Self, RoutingError> {
        let text = text.into();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(RoutingError::EmptySummary);
        }
        let len = trimmed.chars().count();
        if len > MAX_SUMMARY_LEN {
            return Err(RoutingError::SummaryTooLong {
                max: MAX_SUMMARY_LEN,
                got: len,
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the validated summary text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One entry in the routing catalog: a model identity and its summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelCard {
    /// The routing identity, sourced from `Runtime::model()`.
    pub model: ModelId,
    /// What the model is best suited for.
    pub summary: RoutingSummary,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::{MAX_SUMMARY_LEN, ModelCard, RoutingSummary};
    use crate::agent::runtime::routing::error::RoutingError;
    use crate::types::ModelId;

    #[test]
    fn rejects_empty_and_whitespace() {
        assert!(matches!(
            RoutingSummary::new(""),
            Err(RoutingError::EmptySummary)
        ));
        assert!(matches!(
            RoutingSummary::new("   "),
            Err(RoutingError::EmptySummary)
        ));
    }

    #[test]
    fn rejects_over_length() {
        let long = "a".repeat(MAX_SUMMARY_LEN + 1);
        assert!(matches!(
            RoutingSummary::new(long),
            Err(RoutingError::SummaryTooLong { .. })
        ));
    }

    #[test]
    fn trims_and_stores_valid() {
        let s = RoutingSummary::new("  cheap edits  ").expect("valid");
        assert_eq!(s.as_str(), "cheap edits");
    }

    #[test]
    fn card_holds_model_and_summary() {
        let card = ModelCard {
            model: ModelId::custom("deepseek/deepseek-chat").expect("id"),
            summary: RoutingSummary::new("cheap").expect("summary"),
        };
        assert_eq!(card.model.as_str(), "deepseek/deepseek-chat");
        assert_eq!(card.summary.as_str(), "cheap");
    }
}
