//! In-loop hook handlers and their registry.
//!
//! A [`Hook`] is consulted when the binary fires a `hook_callback` control
//! request. [`HookOutput`] is serialized to the binary's camelCase wire shape
//! and returned as the correlated control response. Hooks are registered in a
//! [`HookRegistry`] (see the registration task), which mints the `hook_<n>`
//! callback ids declared in the initialize handshake.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::agent::capabilities::{Capabilities, HookEvent};
use crate::agent::error::AgentError;

/// A hook's control verdict. Only `block` is defined by the protocol today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HookDecision {
    /// Block the action the hook fired on.
    Block,
}

/// The payload a hook returns, serialized as the control response body.
///
/// Field names map to the binary's camelCase wire names; unset fields are
/// omitted so the default is an empty object.
#[derive(Clone, Debug, Default, Serialize)]
pub struct HookOutput {
    /// Whether the agent loop should continue (`continue` on the wire).
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_: Option<bool>,
    /// Suppress the binary's own output for this step.
    #[serde(rename = "suppressOutput", skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    /// A blocking decision, when the hook vetoes the step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<HookDecision>,
    /// A system message injected into the conversation.
    #[serde(rename = "systemMessage", skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    /// Human-readable reason accompanying the decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Input handed to a hook when the binary fires its callback.
#[derive(Clone, Debug)]
pub struct HookInput {
    /// The event that fired this hook.
    pub event: HookEvent,
    /// The tool-use id in scope, when the event is tool-related.
    pub tool_use_id: Option<String>,
    /// The raw event payload from the binary (opaque to the SDK).
    pub data: serde_json::Value,
}

/// A user-supplied hook consulted when its event fires.
#[async_trait]
pub trait Hook: Send + Sync {
    /// Handle the fired event and return the control payload.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the hook fails; the runtime surfaces it to
    /// the binary as an error control response.
    async fn call(&self, input: HookInput) -> Result<HookOutput, AgentError>;
}

/// One registered hook: its event, optional matcher, minted callback id, and
/// handler.
#[derive(Clone)]
struct HookEntry {
    event: HookEvent,
    matcher: Option<String>,
    callback_id: String,
    hook: Arc<dyn Hook>,
}

/// A set of registered hooks, keyed for dispatch by minted callback id.
///
/// Cloning is cheap: handlers are `Arc`-shared. The registry mints a
/// `hook_<n>` id per registration; that id is declared in the initialize
/// handshake and echoed by the binary on each `hook_callback`.
#[derive(Clone, Default)]
pub struct HookRegistry {
    entries: Vec<HookEntry>,
}

impl HookRegistry {
    /// Register `hook` for `event`, optionally narrowed by a `matcher` string.
    ///
    /// Returns `&mut self` for chaining. The minted callback id is
    /// `hook_<index>` in registration order.
    pub fn register(
        &mut self,
        event: HookEvent,
        matcher: Option<String>,
        hook: Arc<dyn Hook>,
    ) -> &mut Self {
        let callback_id = format!("hook_{}", self.entries.len());
        self.entries.push(HookEntry {
            event,
            matcher,
            callback_id,
            hook,
        });
        self
    }

    /// Whether any hooks are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Resolve a callback id to its event and handler.
    #[must_use]
    pub fn lookup(&self, callback_id: &str) -> Option<(HookEvent, Arc<dyn Hook>)> {
        self.entries
            .iter()
            .find(|e| e.callback_id == callback_id)
            .map(|e| (e.event, Arc::clone(&e.hook)))
    }

    /// Build the `hooks` object for the initialize handshake.
    ///
    /// Groups entries by `PascalCase` event name into
    /// `{event: [{matcher?, hookCallbackIds:[…]}]}`. When `caps` lists
    /// supported hook events (non-empty), unsupported events are omitted and a
    /// warning is logged; when `caps` is empty (unknown), all events are
    /// included.
    #[must_use]
    pub fn initialize_payload(&self, caps: &Capabilities) -> serde_json::Value {
        let gate = !caps.supported_hook_events.is_empty();
        let mut map = serde_json::Map::new();
        for entry in &self.entries {
            if gate && !caps.supports_hook(entry.event) {
                tracing::warn!(
                    event = ?entry.event,
                    "hook event not supported by this binary; skipping registration"
                );
                continue;
            }
            let Ok(serde_json::Value::String(name)) = serde_json::to_value(entry.event) else {
                continue;
            };
            let mut obj = serde_json::Map::new();
            if let Some(matcher) = &entry.matcher {
                obj.insert(
                    "matcher".to_string(),
                    serde_json::Value::String(matcher.clone()),
                );
            }
            obj.insert(
                "hookCallbackIds".to_string(),
                serde_json::json!([entry.callback_id]),
            );
            if let Some(arr) = map
                .entry(name)
                .or_insert_with(|| serde_json::Value::Array(Vec::new()))
                .as_array_mut()
            {
                arr.push(serde_json::Value::Object(obj));
            }
        }
        serde_json::Value::Object(map)
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::sync::Arc;

    use super::{Hook, HookDecision, HookInput, HookOutput, HookRegistry};
    use crate::agent::capabilities::{Capabilities, HookEvent};
    use crate::agent::error::AgentError;

    #[test]
    fn empty_output_serializes_to_empty_object() {
        let json = serde_json::to_value(HookOutput::default()).expect("serialize");
        assert_eq!(json, serde_json::json!({}));
    }

    #[test]
    fn output_fields_use_camelcase_wire_names() {
        let output = HookOutput {
            continue_: Some(false),
            suppress_output: Some(true),
            decision: Some(HookDecision::Block),
            system_message: Some("stop".to_string()),
            reason: Some("policy".to_string()),
        };
        let json = serde_json::to_value(output).expect("serialize");
        assert_eq!(json["continue"], false);
        assert_eq!(json["suppressOutput"], true);
        assert_eq!(json["decision"], "block");
        assert_eq!(json["systemMessage"], "stop");
        assert_eq!(json["reason"], "policy");
    }

    struct NoopHook;

    #[async_trait::async_trait]
    impl Hook for NoopHook {
        async fn call(&self, _input: HookInput) -> Result<HookOutput, AgentError> {
            Ok(HookOutput::default())
        }
    }

    #[test]
    fn register_mints_sequential_callback_ids() {
        let mut reg = HookRegistry::default();
        reg.register(
            HookEvent::PreToolUse,
            Some("Bash".to_string()),
            Arc::new(NoopHook),
        );
        reg.register(HookEvent::Stop, None, Arc::new(NoopHook));
        assert!(reg.lookup("hook_0").is_some());
        let (event, _hook) = reg.lookup("hook_1").expect("second hook");
        assert_eq!(event, HookEvent::Stop);
        assert!(reg.lookup("hook_2").is_none());
    }

    #[test]
    fn initialize_payload_groups_by_event_with_callback_ids() {
        let mut reg = HookRegistry::default();
        reg.register(
            HookEvent::PreToolUse,
            Some("Bash".to_string()),
            Arc::new(NoopHook),
        );
        let caps = Capabilities::default(); // empty => no gating
        let value = reg.initialize_payload(&caps);
        let entries = value["PreToolUse"].as_array().expect("array");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["matcher"], "Bash");
        assert_eq!(entries[0]["hookCallbackIds"][0], "hook_0");
    }

    #[test]
    fn initialize_payload_omits_unsupported_events_when_caps_known() {
        let mut reg = HookRegistry::default();
        reg.register(HookEvent::PreToolUse, None, Arc::new(NoopHook));
        reg.register(HookEvent::Stop, None, Arc::new(NoopHook));
        let mut supported = std::collections::HashSet::new();
        supported.insert(HookEvent::PreToolUse);
        let caps = Capabilities {
            supported_hook_events: supported,
            ..Capabilities::default()
        };
        let value = reg.initialize_payload(&caps);
        assert!(value.get("PreToolUse").is_some());
        assert!(
            value.get("Stop").is_none(),
            "unsupported event must be omitted"
        );
    }

    #[test]
    fn empty_registry_yields_empty_payload() {
        let reg = HookRegistry::default();
        let value = reg.initialize_payload(&Capabilities::default());
        assert_eq!(value, serde_json::json!({}));
    }
}
