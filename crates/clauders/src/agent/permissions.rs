//! Permission control for the agent.
//!
//! Defines the [`PermissionMode`] data enum forwarded to the binary on the
//! `set_permission_mode` control request and carried in [`crate::agent::Options`].
//! Also defines [`PermissionContext`], [`PermissionDecision`], and the
//! [`PermissionPolicy`] trait used by the runtime's in-loop permission handler.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::agent::error::AgentError;

/// How the binary should gate tool use for a session.
///
/// The value is forwarded verbatim to the binary's `set_permission_mode`
/// control request. Wire names are the camelCase strings the binary expects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    /// Prompt per the binary's default policy.
    #[default]
    #[serde(rename = "default")]
    Default,
    /// Auto-accept file edits.
    #[serde(rename = "acceptEdits")]
    AcceptEdits,
    /// Planning mode — propose without executing.
    #[serde(rename = "plan")]
    Plan,
    /// Bypass all permission prompts.
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
}

/// Context for a tool-permission request, mirrored from the inbound
/// `can_use_tool` control request. All fields are optional: the binary
/// populates whichever it has for the call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PermissionContext {
    /// Id of the tool-use block this request gates.
    pub tool_use_id: Option<String>,
    /// Id of the (sub)agent issuing the tool call.
    pub agent_id: Option<String>,
    /// Path the call is blocked on, when applicable.
    pub blocked_path: Option<String>,
    /// Why the binary is asking (its own pre-decision reason).
    pub decision_reason: Option<String>,
    /// Short human title for the request.
    pub title: Option<String>,
    /// Display name of the tool.
    pub display_name: Option<String>,
    /// Longer human description of the request.
    pub description: Option<String>,
}

/// A policy's verdict on a tool call.
///
/// Serialized into the `can_use_tool` control response via
/// [`PermissionDecision::into_response_value`] using the binary's wire shape:
/// `{"behavior":"allow","updatedInput":…}` or
/// `{"behavior":"deny","message":…}`.
#[derive(Clone, Debug)]
pub enum PermissionDecision {
    /// Allow the call, optionally rewriting its input.
    Allow {
        /// Replacement input; `None` keeps the original input unchanged.
        updated_input: Option<serde_json::Value>,
    },
    /// Deny the call with a human-readable reason.
    Deny {
        /// Why the call was denied.
        message: String,
    },
}

impl PermissionDecision {
    /// Render this decision as the `response` payload of the control response.
    ///
    /// On `Allow` without a rewrite, `original_input` is echoed as
    /// `updatedInput` (the binary always expects the field present).
    #[must_use]
    pub fn into_response_value(self, original_input: &serde_json::Value) -> serde_json::Value {
        match self {
            Self::Allow { updated_input } => serde_json::json!({
                "behavior": "allow",
                "updatedInput": updated_input.unwrap_or_else(|| original_input.clone()),
            }),
            Self::Deny { message } => serde_json::json!({
                "behavior": "deny",
                "message": message,
            }),
        }
    }
}

/// A user-supplied policy consulted on each `can_use_tool` request.
///
/// Registered via [`crate::agent::Options`] and consulted by the runtime's
/// background reader; the returned [`PermissionDecision`] is sent back to the
/// binary as the correlated control response.
#[async_trait]
pub trait PermissionPolicy: Send + Sync {
    /// Decide whether `tool` may run with `input`.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the policy cannot reach a decision; the
    /// runtime surfaces it to the binary as an error control response.
    async fn can_use_tool(
        &self,
        tool: &str,
        input: &serde_json::Value,
        ctx: PermissionContext,
    ) -> Result<PermissionDecision, AgentError>;
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{PermissionDecision, PermissionMode};

    #[test]
    fn default_is_default_variant() {
        assert_eq!(PermissionMode::default(), PermissionMode::Default);
    }

    #[test]
    fn serializes_to_wire_string() {
        let json = serde_json::to_string(&PermissionMode::AcceptEdits).expect("serialize");
        assert_eq!(json, "\"acceptEdits\"");
        let json = serde_json::to_string(&PermissionMode::BypassPermissions).expect("serialize");
        assert_eq!(json, "\"bypassPermissions\"");
    }

    #[test]
    fn round_trips_plan_variant() {
        let back: PermissionMode = serde_json::from_str("\"plan\"").expect("deserialize");
        assert_eq!(back, PermissionMode::Plan);
    }

    #[test]
    fn allow_without_rewrite_echoes_original_input() {
        let original = serde_json::json!({ "cmd": "ls" });
        let value = PermissionDecision::Allow {
            updated_input: None,
        }
        .into_response_value(&original);
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["updatedInput"], serde_json::json!({ "cmd": "ls" }));
    }

    #[test]
    fn allow_with_rewrite_uses_updated_input() {
        let original = serde_json::json!({ "cmd": "ls" });
        let rewritten = serde_json::json!({ "cmd": "ls -la" });
        let value = PermissionDecision::Allow {
            updated_input: Some(rewritten.clone()),
        }
        .into_response_value(&original);
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["updatedInput"], rewritten);
    }

    #[test]
    fn deny_carries_message() {
        let value = PermissionDecision::Deny {
            message: "blocked by policy".to_string(),
        }
        .into_response_value(&serde_json::json!({}));
        assert_eq!(value["behavior"], "deny");
        assert_eq!(value["message"], "blocked by policy");
    }
}
