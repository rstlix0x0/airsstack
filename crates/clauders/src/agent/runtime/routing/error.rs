//! Errors raised while constructing or driving a routing runtime.

use crate::types::ModelId;

/// A failure in routing configuration or a routing decision.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum RoutingError {
    /// A routing summary was empty after trimming.
    #[error("routing summary must not be empty")]
    EmptySummary,
    /// A routing summary exceeded the length bound.
    #[error("routing summary too long: {got} chars exceeds max {max}")]
    SummaryTooLong {
        /// The maximum allowed length, in characters.
        max: usize,
        /// The rejected length, in characters.
        got: usize,
    },
    /// A target runtime exposed no model identity (`Runtime::model()` was `None`).
    #[error("routing target has no model identity")]
    MissingModelId,
    /// Two targets resolved to the same model identity.
    #[error("duplicate routing target model: {0}")]
    DuplicateModel(ModelId),
    /// The classifier run failed or produced no usable reply.
    #[error("classification failed: {0}")]
    Classify(String),
    /// The classifier reply matched no candidate model id.
    #[error("classifier reply matched no candidate: {reply}")]
    Parse {
        /// The unmatched reply text.
        reply: String,
    },
    /// A control operation was called before any `run()` selected a target.
    #[error("no active routing target; call run() first")]
    NoActiveTarget,
}

#[cfg(test)]
mod tests {
    use super::RoutingError;

    #[test]
    fn display_messages_are_descriptive() {
        assert!(RoutingError::EmptySummary.to_string().contains("empty"));
        assert!(
            RoutingError::SummaryTooLong { max: 512, got: 600 }
                .to_string()
                .contains("600")
        );
        assert!(RoutingError::NoActiveTarget.to_string().contains("run()"));
    }
}
