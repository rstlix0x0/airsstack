//! The routing decision seam: the `Classifier` trait and its model-backed impl.

use std::future::poll_fn;

use async_trait::async_trait;

use crate::agent::message::Message;
use crate::agent::runtime::Runtime;
use crate::agent::types::Prompt;
use crate::types::ModelId;

use super::card::ModelCard;
use super::error::RoutingError;

/// Chooses one target model for a prompt from a catalog of candidates.
#[async_trait]
pub trait Classifier: Send + Sync {
    /// Return the [`ModelId`] of the catalog entry best suited to `prompt`.
    ///
    /// The returned id must be one of the ids in `catalog`.
    ///
    /// # Errors
    /// Returns [`RoutingError`] if the decision cannot be produced (transport
    /// failure or an unparseable reply); the caller then applies its fallback.
    async fn classify(
        &self,
        prompt: &Prompt,
        catalog: &[ModelCard],
    ) -> Result<ModelId, RoutingError>;
}

/// A [`Classifier`] that asks a cheap model — driven through any [`Runtime`] —
/// to pick the best target.
pub struct RuntimeClassifier<R: Runtime> {
    runtime: R,
}

impl<R: Runtime> RuntimeClassifier<R> {
    /// Wrap the runtime that will perform classification queries.
    pub const fn new(runtime: R) -> Self {
        Self { runtime }
    }

    /// Render the fixed selection prompt from the catalog and task prompt.
    fn selection_prompt(prompt: &Prompt, catalog: &[ModelCard]) -> String {
        let mut body = String::from(
            "You are a model router. From the candidate models below, choose the single \
             best one for the task. Reply with ONLY that model's id, nothing else.\n\n\
             Candidates:\n",
        );
        for card in catalog {
            body.push_str("- ");
            body.push_str(card.model.as_str());
            body.push_str(": ");
            body.push_str(card.summary.as_str());
            body.push('\n');
        }
        body.push_str("\nTask:\n");
        body.push_str(prompt.as_str());
        body.push_str("\n\nModel id:");
        body
    }
}

#[async_trait]
impl<R: Runtime> Classifier for RuntimeClassifier<R> {
    async fn classify(
        &self,
        prompt: &Prompt,
        catalog: &[ModelCard],
    ) -> Result<ModelId, RoutingError> {
        let selection = Self::selection_prompt(prompt, catalog);
        let mut stream = self
            .runtime
            .run(Prompt::new(selection))
            .await
            .map_err(|e| RoutingError::Classify(e.to_string()))?;

        // Drain the stream to its terminal result. `futures_util` is a test-only
        // dependency, so poll the `futures_core::Stream` directly here.
        let mut reply: Option<String> = None;
        while let Some(item) = poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
            match item {
                Ok(Message::Result(r)) => reply = Some(r.result),
                Ok(_) => {}
                Err(e) => return Err(RoutingError::Classify(e.to_string())),
            }
        }
        let reply = reply
            .ok_or_else(|| RoutingError::Classify("classifier produced no result".to_string()))?;

        catalog
            .iter()
            .find(|card| reply.contains(card.model.as_str()))
            .map(|card| card.model.clone())
            .ok_or(RoutingError::Parse { reply })
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{Classifier, RuntimeClassifier};
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::runtime::routing::card::{ModelCard, RoutingSummary};
    use crate::agent::runtime::routing::error::RoutingError;
    use crate::agent::types::{Prompt, SessionId};
    use crate::types::ModelId;

    fn card(id: &str, summary: &str) -> ModelCard {
        ModelCard {
            model: ModelId::custom(id).expect("id"),
            summary: RoutingSummary::new(summary).expect("summary"),
        }
    }

    fn result(text: &str) -> Message {
        Message::Result(ResultMessage {
            result: text.into(),
            structured_output: None,
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })
    }

    fn catalog() -> Vec<ModelCard> {
        vec![
            card("deepseek/deepseek-chat", "cheap"),
            card("anthropic/claude-opus-4-7", "advanced"),
        ]
    }

    #[tokio::test]
    async fn returns_the_named_model() {
        let judge = MockRuntime::new(vec![vec![result("anthropic/claude-opus-4-7")]]);
        let clf = RuntimeClassifier::new(judge);
        let id = clf
            .classify(&Prompt::new("hard"), &catalog())
            .await
            .expect("classify");
        assert_eq!(id.as_str(), "anthropic/claude-opus-4-7");
    }

    #[tokio::test]
    async fn unknown_reply_is_a_parse_error() {
        let judge = MockRuntime::new(vec![vec![result("gpt-4")]]);
        let clf = RuntimeClassifier::new(judge);
        let err = clf
            .classify(&Prompt::new("x"), &catalog())
            .await
            .expect_err("no match");
        assert!(matches!(err, RoutingError::Parse { .. }));
    }

    #[tokio::test]
    async fn empty_stream_is_a_classify_error() {
        let judge = MockRuntime::new(vec![]); // no scripted turn -> empty stream
        let clf = RuntimeClassifier::new(judge);
        let err = clf
            .classify(&Prompt::new("x"), &catalog())
            .await
            .expect_err("no result");
        assert!(matches!(err, RoutingError::Classify(_)));
    }
}
