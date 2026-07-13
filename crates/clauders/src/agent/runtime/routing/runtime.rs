//! The routing meta-runtime: chooses one backend per `run()` via a classifier.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;

use crate::agent::capabilities::{Capabilities, HookEvent};
use crate::agent::error::AgentError;
use crate::agent::permissions::PermissionMode;
use crate::agent::runtime::Runtime;
use crate::agent::stream::MessageStream;
use crate::agent::types::{McpStatus, Prompt};
use crate::types::ModelId;

use super::card::ModelCard;
use super::classifier::Classifier;
use super::error::RoutingError;

/// The protocol-version marker a routing runtime advertises.
const ROUTING_PROTOCOL_VERSION: &str = "routing-1.0";

/// A [`Runtime`] that routes each `run()` to one of several backend runtimes,
/// choosing the target with a [`Classifier`].
pub struct RoutingRuntime<C: Classifier> {
    classifier: C,
    targets: HashMap<ModelId, Arc<dyn Runtime>>,
    catalog: Vec<ModelCard>,
    fallback: ModelId,
    active: Arc<Mutex<Option<ModelId>>>,
    capabilities: Capabilities,
}

impl<C: Classifier> RoutingRuntime<C> {
    /// Assemble a routing runtime from already-validated parts.
    ///
    /// Prefer [`RoutingRuntime::builder`]. This assumes `fallback` is present
    /// in `targets` and every catalog entry has a matching target.
    pub(super) fn from_parts(
        classifier: C,
        targets: HashMap<ModelId, Arc<dyn Runtime>>,
        catalog: Vec<ModelCard>,
        fallback: ModelId,
    ) -> Self {
        let capabilities = intersect_capabilities(targets.values());
        Self {
            classifier,
            targets,
            catalog,
            fallback,
            active: Arc::new(Mutex::new(None)),
            capabilities,
        }
    }

    /// Read the active target id, recovering from a poisoned lock.
    fn active_target(&self) -> Option<ModelId> {
        self.active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Record `id` as the active target for subsequent control ops.
    fn set_active(&self, id: ModelId) {
        *self.active.lock().unwrap_or_else(PoisonError::into_inner) = Some(id);
    }

    /// The backend for the active target, or an error if none is selected.
    fn active_delegate(&self) -> Result<Arc<dyn Runtime>, AgentError> {
        let id = self.active_target().ok_or_else(no_active_target)?;
        self.targets
            .get(&id)
            .map(Arc::clone)
            .ok_or_else(no_active_target)
    }
}

/// Map the "no active target" condition onto the agent-facing error.
fn no_active_target() -> AgentError {
    AgentError::Protocol {
        detail: RoutingError::NoActiveTarget.to_string(),
    }
}

/// The conservative capability floor across all targets: the set intersection
/// of their supported hooks and control methods, under the routing marker.
fn intersect_capabilities<'a>(
    targets: impl IntoIterator<Item = &'a Arc<dyn Runtime>>,
) -> Capabilities {
    let mut hooks: Option<HashSet<HookEvent>> = None;
    let mut methods: Option<HashSet<String>> = None;
    for target in targets {
        let caps = target.capabilities();
        hooks = Some(hooks.map_or_else(
            || caps.supported_hook_events.clone(),
            |acc| {
                acc.intersection(&caps.supported_hook_events)
                    .copied()
                    .collect()
            },
        ));
        methods = Some(methods.map_or_else(
            || caps.supported_control_methods.clone(),
            |acc| {
                acc.intersection(&caps.supported_control_methods)
                    .cloned()
                    .collect()
            },
        ));
    }
    Capabilities {
        protocol_version: ROUTING_PROTOCOL_VERSION.to_string(),
        supported_hook_events: hooks.unwrap_or_default(),
        supported_control_methods: methods.unwrap_or_default(),
    }
}

#[async_trait]
impl<C: Classifier> Runtime for RoutingRuntime<C> {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
        // Classify, then validate the pick is a known target; any error or an
        // unknown id degrades to the fallback rather than failing the turn.
        let id = match self.classifier.classify(&prompt, &self.catalog).await {
            Ok(id) if self.targets.contains_key(&id) => id,
            _ => self.fallback.clone(),
        };
        self.set_active(id.clone());
        // `id` is guaranteed present: it is either a validated key or the
        // fallback, which `from_parts` guarantees is in `targets`.
        let target = self
            .targets
            .get(&id)
            .map_or_else(|| Arc::clone(&self.targets[&self.fallback]), Arc::clone);
        target.run(prompt).await
    }

    async fn interrupt(&self) -> Result<(), AgentError> {
        self.active_delegate()?.interrupt().await
    }

    async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
        self.active_delegate()?.set_model(model).await
    }

    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
        self.active_delegate()?.set_permission_mode(mode).await
    }

    async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
        self.active_delegate()?.mcp_status().await
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::collections::HashMap;
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_util::StreamExt;

    use super::{ROUTING_PROTOCOL_VERSION, RoutingRuntime, intersect_capabilities};
    use crate::agent::capabilities::Capabilities;
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::runtime::Runtime;
    use crate::agent::runtime::mock::{ControlCall, MockRuntime};
    use crate::agent::runtime::routing::card::{ModelCard, RoutingSummary};
    use crate::agent::runtime::routing::classifier::Classifier;
    use crate::agent::runtime::routing::error::RoutingError;
    use crate::agent::stream::MessageStream;
    use crate::agent::types::{Prompt, SessionId};
    use crate::types::ModelId;

    // --- test doubles / helpers -------------------------------------------------

    struct MockClassifier {
        pick: Result<ModelId, RoutingError>,
    }

    #[async_trait]
    impl Classifier for MockClassifier {
        async fn classify(
            &self,
            _prompt: &Prompt,
            _catalog: &[ModelCard],
        ) -> Result<ModelId, RoutingError> {
            self.pick.clone()
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

    fn id(s: &str) -> ModelId {
        ModelId::custom(s).expect("id")
    }

    fn summary() -> RoutingSummary {
        RoutingSummary::new("desc").expect("summary")
    }

    async fn drain(stream: &mut MessageStream) -> String {
        let mut last = String::new();
        while let Some(item) = stream.next().await {
            if let Message::Result(r) = item.expect("frame") {
                last = r.result;
            }
        }
        last
    }

    /// A two-target router (deepseek + claude), fallback = claude, with a
    /// scripted classifier decision.
    fn routing(pick: Result<ModelId, RoutingError>) -> RoutingRuntime<MockClassifier> {
        let cheap = id("deepseek/deepseek-chat");
        let adv = id("anthropic/claude-opus-4-7");
        let mut targets: HashMap<ModelId, Arc<dyn Runtime>> = HashMap::new();
        targets.insert(
            cheap.clone(),
            Arc::new(MockRuntime::new(vec![vec![result("cheap-ran")]]).with_model(cheap.clone())),
        );
        targets.insert(
            adv.clone(),
            Arc::new(MockRuntime::new(vec![vec![result("adv-ran")]]).with_model(adv.clone())),
        );
        let catalog = vec![
            ModelCard {
                model: cheap,
                summary: summary(),
            },
            ModelCard {
                model: adv.clone(),
                summary: summary(),
            },
        ];
        RoutingRuntime::from_parts(MockClassifier { pick }, targets, catalog, adv)
    }

    // --- capability intersection ------------------------------------------------

    fn caps(methods: &[&str]) -> Capabilities {
        let mut c = Capabilities::default();
        for m in methods {
            c.supported_control_methods.insert((*m).to_string());
        }
        c
    }

    #[test]
    fn intersection_is_the_common_control_methods() {
        let a: Arc<dyn Runtime> =
            Arc::new(MockRuntime::new(vec![]).with_capabilities(caps(&["set_model", "interrupt"])));
        let b: Arc<dyn Runtime> =
            Arc::new(MockRuntime::new(vec![]).with_capabilities(caps(&["set_model"])));
        let targets = [a, b];
        let merged = intersect_capabilities(targets.iter());
        assert_eq!(merged.protocol_version, ROUTING_PROTOCOL_VERSION);
        assert!(merged.supports_control("set_model"));
        assert!(!merged.supports_control("interrupt"));
    }

    // --- routing + fallback -----------------------------------------------------

    #[tokio::test]
    async fn routes_to_the_classified_target() {
        let rt = routing(Ok(id("deepseek/deepseek-chat")));
        let mut stream = rt.run(Prompt::new("easy")).await.expect("run");
        assert_eq!(drain(&mut stream).await, "cheap-ran");
    }

    #[tokio::test]
    async fn unknown_pick_falls_back() {
        let rt = routing(Ok(id("gpt-4")));
        let mut stream = rt.run(Prompt::new("x")).await.expect("run");
        assert_eq!(drain(&mut stream).await, "adv-ran");
    }

    #[tokio::test]
    async fn classifier_error_falls_back() {
        let rt = routing(Err(RoutingError::Classify("boom".into())));
        let mut stream = rt.run(Prompt::new("x")).await.expect("run");
        assert_eq!(drain(&mut stream).await, "adv-ran");
    }

    // --- control-op delegation --------------------------------------------------

    #[tokio::test]
    async fn control_op_before_run_errors_then_delegates_after() {
        let cheap = id("deepseek/deepseek-chat");
        let cheap_mock =
            Arc::new(MockRuntime::new(vec![vec![result("cheap-ran")]]).with_model(cheap.clone()));
        let mut targets: HashMap<ModelId, Arc<dyn Runtime>> = HashMap::new();
        targets.insert(cheap.clone(), cheap_mock.clone() as Arc<dyn Runtime>);
        let catalog = vec![ModelCard {
            model: cheap.clone(),
            summary: summary(),
        }];
        let rt = RoutingRuntime::from_parts(
            MockClassifier {
                pick: Ok(cheap.clone()),
            },
            targets,
            catalog,
            cheap,
        );

        // Before any run(): NoActiveTarget -> Err.
        assert!(rt.interrupt().await.is_err());

        // After a run(): the control op reaches the selected target.
        let _ = rt.run(Prompt::new("hi")).await.expect("run");
        rt.interrupt().await.expect("interrupt");
        assert_eq!(cheap_mock.calls(), vec![ControlCall::Interrupt]);
    }
}
