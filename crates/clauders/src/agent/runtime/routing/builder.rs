//! The type-state builder for [`RoutingRuntime`].

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::agent::runtime::Runtime;
use crate::types::ModelId;

use super::card::{ModelCard, RoutingSummary};
use super::classifier::Classifier;
use super::error::RoutingError;
use super::runtime::RoutingRuntime;

/// Builder state: a fallback target has not yet been set.
pub struct NeedsFallback;
/// Builder state: a fallback target is set; `build` is available.
pub struct Ready;

/// A type-state builder for [`RoutingRuntime`].
///
/// The classifier is required up front (via [`RoutingRuntime::builder`]); a
/// fallback target is required before [`build`](RoutingRuntimeBuilder::build)
/// becomes callable. Additional targets are optional. Each target's id is read
/// from [`Runtime::model`]; the summary describes it to the classifier.
pub struct RoutingRuntimeBuilder<C, State> {
    classifier: C,
    targets: Vec<(Arc<dyn Runtime>, RoutingSummary)>,
    fallback: Option<Arc<dyn Runtime>>,
    _state: PhantomData<State>,
}

impl<C: Classifier> RoutingRuntime<C> {
    /// Start building a routing runtime around `classifier`.
    pub fn builder(classifier: C) -> RoutingRuntimeBuilder<C, NeedsFallback> {
        RoutingRuntimeBuilder {
            classifier,
            targets: Vec::new(),
            fallback: None,
            _state: PhantomData,
        }
    }
}

impl<C, State> RoutingRuntimeBuilder<C, State> {
    /// Register an additional routing target.
    #[must_use]
    pub fn target(mut self, runtime: impl Runtime + 'static, summary: RoutingSummary) -> Self {
        self.targets.push((Arc::new(runtime), summary));
        self
    }
}

impl<C> RoutingRuntimeBuilder<C, NeedsFallback> {
    /// Set the fallback target — the backend used when classification fails or
    /// names an unknown model — transitioning the builder to [`Ready`].
    #[must_use]
    pub fn fallback_target(
        self,
        runtime: impl Runtime + 'static,
        summary: RoutingSummary,
    ) -> RoutingRuntimeBuilder<C, Ready> {
        let fallback: Arc<dyn Runtime> = Arc::new(runtime);
        let mut targets = self.targets;
        targets.push((Arc::clone(&fallback), summary));
        RoutingRuntimeBuilder {
            classifier: self.classifier,
            targets,
            fallback: Some(fallback),
            _state: PhantomData,
        }
    }
}

impl<C: Classifier> RoutingRuntimeBuilder<C, Ready> {
    /// Validate the targets and assemble the [`RoutingRuntime`].
    ///
    /// # Errors
    /// [`RoutingError::MissingModelId`] if any target exposes no model identity;
    /// [`RoutingError::DuplicateModel`] if two targets resolve to the same id.
    pub fn build(self) -> Result<RoutingRuntime<C>, RoutingError> {
        // The `Ready` state guarantees a fallback; treat its absence defensively.
        let Some(fallback) = self.fallback else {
            return Err(RoutingError::MissingModelId);
        };
        let fallback_id = fallback
            .model()
            .ok_or(RoutingError::MissingModelId)?
            .clone();

        let mut targets: HashMap<ModelId, Arc<dyn Runtime>> = HashMap::new();
        let mut catalog: Vec<ModelCard> = Vec::new();
        for (runtime, summary) in self.targets {
            let id = runtime.model().ok_or(RoutingError::MissingModelId)?.clone();
            if targets.contains_key(&id) {
                return Err(RoutingError::DuplicateModel(id));
            }
            catalog.push(ModelCard {
                model: id.clone(),
                summary,
            });
            targets.insert(id, runtime);
        }

        Ok(RoutingRuntime::from_parts(
            self.classifier,
            targets,
            catalog,
            fallback_id,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use async_trait::async_trait;

    use super::RoutingRuntime;
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::runtime::routing::card::{ModelCard, RoutingSummary};
    use crate::agent::runtime::routing::classifier::Classifier;
    use crate::agent::runtime::routing::error::RoutingError;
    use crate::agent::types::Prompt;
    use crate::types::ModelId;

    struct AnyClassifier;

    #[async_trait]
    impl Classifier for AnyClassifier {
        async fn classify(
            &self,
            _prompt: &Prompt,
            _catalog: &[ModelCard],
        ) -> Result<ModelId, RoutingError> {
            Err(RoutingError::Classify("unused".to_string()))
        }
    }

    fn mock(id: &str) -> MockRuntime {
        MockRuntime::new(vec![]).with_model(ModelId::custom(id).expect("id"))
    }

    fn summary() -> RoutingSummary {
        RoutingSummary::new("desc").expect("summary")
    }

    #[test]
    fn builds_with_valid_targets() {
        let built = RoutingRuntime::builder(AnyClassifier)
            .target(mock("deepseek/deepseek-chat"), summary())
            .fallback_target(mock("anthropic/claude-opus-4-7"), summary())
            .build();
        assert!(built.is_ok());
    }

    #[test]
    fn rejects_target_without_model() {
        let built = RoutingRuntime::builder(AnyClassifier)
            .target(MockRuntime::new(vec![]), summary()) // no with_model -> model() is None
            .fallback_target(mock("anthropic/claude-opus-4-7"), summary())
            .build();
        assert!(matches!(built, Err(RoutingError::MissingModelId)));
    }

    #[test]
    fn rejects_duplicate_model() {
        let built = RoutingRuntime::builder(AnyClassifier)
            .target(mock("deepseek/deepseek-chat"), summary())
            .fallback_target(mock("deepseek/deepseek-chat"), summary())
            .build();
        assert!(matches!(built, Err(RoutingError::DuplicateModel(_))));
    }

    #[test]
    fn fallback_is_required_at_compile_time() {
        // The following does not compile: `build` exists only on the `Ready`
        // state, reached by `fallback_target`.
        //
        //   RoutingRuntime::builder(AnyClassifier)
        //       .target(mock("deepseek/deepseek-chat"), summary())
        //       .build();   // ^ no method named `build`
    }
}
