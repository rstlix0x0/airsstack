//! The tool-permission request context and a policy's verdict.

use serde::Deserialize;

use super::update::PermissionUpdate;
use crate::agent::error::AgentError;

/// The user-configured ask rule that forced a permission prompt.
///
/// Present when a `permissions.ask` rule caused the prompt but the request
/// carries the tool's own richer reason. A host running its own
/// auto-approval should treat a request carrying this as user-intended.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MatchedAskRule {
    /// Which settings source supplied the rule.
    pub source: String,
    /// The tool the rule names.
    pub tool_name: String,
    /// The argument pattern the rule matched, when narrower than the tool.
    #[serde(default)]
    pub rule_content: Option<String>,
}

/// Context for a tool-permission request, mirrored from the inbound
/// `can_use_tool` control request.
///
/// `request_id` is always populated; every other field is optional and
/// carries whichever the binary supplied for the call.
///
/// Non-exhaustive: `sdk.d.ts`'s control request for this call carries more
/// fields than this release models (`decision_reason_type`,
/// `classifier_approvable`, `suppress_always_allow_rule`,
/// `requires_user_interaction`), and this type is only ever constructed
/// inside this crate, so adding one is not a breaking change for a
/// downstream caller.
///
/// Derives `PartialEq` but not `Eq`: `suggestions` holds
/// [`PermissionUpdate`], which carries a `serde_json::Value` in its
/// `Unknown` arm and `Value` does not implement `Eq`.
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq)]
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
    /// Rule updates the binary suggests the policy may apply.
    ///
    /// Wire name `permission_suggestions`; named for the callback argument
    /// the official SDKs pass.
    pub suggestions: Vec<PermissionUpdate>,
    /// The user-configured ask rule that forced this prompt, when one did.
    pub matched_ask_rule: Option<MatchedAskRule>,
    /// Correlation id of the control request this decision answers.
    pub request_id: String,
}

/// A policy's verdict on a tool call.
///
/// Serialized into the `can_use_tool` control response via
/// [`PermissionDecision::into_response_value`] using the binary's wire shape.
#[derive(Clone, Debug)]
pub enum PermissionDecision {
    /// Allow the call, optionally rewriting its input.
    Allow {
        /// Replacement input; `None` keeps the original input unchanged.
        updated_input: Option<serde_json::Value>,
        /// Rule updates to persist for the rest of the session.
        updated_permissions: Vec<PermissionUpdate>,
    },
    /// Deny the call with a human-readable reason.
    Deny {
        /// Why the call was denied.
        message: String,
        /// When `true`, the runtime aborts the whole turn, not just this call.
        interrupt: bool,
        /// Rule updates to persist for the rest of the session.
        updated_permissions: Vec<PermissionUpdate>,
    },
}

impl PermissionDecision {
    /// Allow the call unchanged, with no rule updates.
    #[must_use]
    pub const fn allow() -> Self {
        Self::Allow {
            updated_input: None,
            updated_permissions: Vec::new(),
        }
    }

    /// Allow the call with rewritten input, with no rule updates.
    #[must_use]
    pub const fn allow_with(input: serde_json::Value) -> Self {
        Self::Allow {
            updated_input: Some(input),
            updated_permissions: Vec::new(),
        }
    }

    /// Deny the call without aborting the turn, with no rule updates.
    #[must_use]
    pub fn deny(message: impl Into<String>) -> Self {
        Self::Deny {
            message: message.into(),
            interrupt: false,
            updated_permissions: Vec::new(),
        }
    }

    /// Deny the call and abort the whole turn, with no rule updates.
    #[must_use]
    pub fn deny_interrupt(message: impl Into<String>) -> Self {
        Self::Deny {
            message: message.into(),
            interrupt: true,
            updated_permissions: Vec::new(),
        }
    }

    /// The rule updates this decision carries, on either variant.
    #[must_use]
    pub fn updated_permissions(&self) -> &[PermissionUpdate] {
        match self {
            Self::Allow {
                updated_permissions,
                ..
            }
            | Self::Deny {
                updated_permissions,
                ..
            } => updated_permissions,
        }
    }

    /// Render this decision as the `response` payload of the control response.
    ///
    /// On `Allow` without a rewrite, `original_input` is echoed as
    /// `updatedInput` (the binary always expects the field present).
    /// `updatedPermissions` is emitted only when non-empty.
    ///
    /// # Errors
    /// Returns [`AgentError::Protocol`] if `updated_permissions` fails to
    /// serialize. A failure here must not be swallowed: doing so would drop
    /// every update in the array behind a silent `null` on the wire.
    pub fn into_response_value(
        self,
        original_input: &serde_json::Value,
    ) -> Result<serde_json::Value, AgentError> {
        match self {
            Self::Allow {
                updated_input,
                updated_permissions,
            } => {
                let mut value = serde_json::json!({
                    "behavior": "allow",
                    "updatedInput": updated_input.unwrap_or_else(|| original_input.clone()),
                });
                attach_updates(&mut value, &updated_permissions)?;
                Ok(value)
            }
            Self::Deny {
                message,
                interrupt,
                updated_permissions,
            } => {
                let mut value = serde_json::json!({
                    "behavior": "deny",
                    "message": message,
                    "interrupt": interrupt,
                });
                attach_updates(&mut value, &updated_permissions)?;
                Ok(value)
            }
        }
    }
}

/// Attach a non-empty `updatedPermissions` array to a response value.
fn attach_updates(
    value: &mut serde_json::Value,
    updates: &[PermissionUpdate],
) -> Result<(), AgentError> {
    if !updates.is_empty() {
        let serialized = serde_json::to_value(updates).map_err(|e| AgentError::Protocol {
            detail: format!("failed to serialize updatedPermissions: {e}"),
        })?;
        value["updatedPermissions"] = serialized;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::PermissionDecision;
    use super::PermissionUpdate;
    use crate::agent::permissions::{
        PermissionBehavior, PermissionRuleValue, PermissionUpdateDestination,
    };

    #[test]
    fn allow_without_rewrite_echoes_original_input() {
        let original = serde_json::json!({ "cmd": "ls" });
        let value = PermissionDecision::allow()
            .into_response_value(&original)
            .expect("serialize");
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["updatedInput"], serde_json::json!({ "cmd": "ls" }));
    }

    #[test]
    fn allow_with_rewrite_uses_updated_input() {
        let original = serde_json::json!({ "cmd": "ls" });
        let rewritten = serde_json::json!({ "cmd": "ls -la" });
        let value = PermissionDecision::allow_with(rewritten.clone())
            .into_response_value(&original)
            .expect("serialize");
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["updatedInput"], rewritten);
    }

    #[test]
    fn deny_carries_message() {
        let value = PermissionDecision::deny("blocked by policy")
            .into_response_value(&serde_json::json!({}))
            .expect("serialize");
        assert_eq!(value["behavior"], "deny");
        assert_eq!(value["message"], "blocked by policy");
    }

    #[test]
    fn deny_constructor_defaults_interrupt_false_no_updates() {
        let decision = PermissionDecision::deny("nope");
        assert!(decision.updated_permissions().is_empty());
        let value = decision
            .into_response_value(&serde_json::json!({}))
            .expect("serialize");
        assert_eq!(value["behavior"], "deny");
        assert_eq!(value["message"], "nope");
        assert_eq!(value["interrupt"], false);
        assert!(value.get("updatedPermissions").is_none());
    }

    #[test]
    fn deny_interrupt_constructor_sets_interrupt_true() {
        let value = PermissionDecision::deny_interrupt("stop")
            .into_response_value(&serde_json::json!({}))
            .expect("serialize");
        assert_eq!(value["interrupt"], true);
    }

    #[test]
    fn allow_constructor_echoes_input_and_omits_empty_updates() {
        let original = serde_json::json!({ "cmd": "ls" });
        let value = PermissionDecision::allow()
            .into_response_value(&original)
            .expect("serialize");
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["updatedInput"]["cmd"], "ls");
        assert!(value.get("updatedPermissions").is_none());
    }

    #[test]
    fn updated_permissions_serialize_into_the_response() {
        let update = PermissionUpdate::AddRules {
            rules: vec![PermissionRuleValue {
                tool_name: "Bash".to_string(),
                rule_content: None,
            }],
            behavior: PermissionBehavior::Allow,
            destination: PermissionUpdateDestination::Session,
        };
        let decision = PermissionDecision::Deny {
            message: "denied".to_string(),
            interrupt: false,
            updated_permissions: vec![update],
        };
        let value = decision
            .into_response_value(&serde_json::json!({}))
            .expect("serialize");
        assert_eq!(
            value["updatedPermissions"][0]["rules"][0]["toolName"],
            "Bash"
        );
        assert_eq!(value["updatedPermissions"][0]["behavior"], "allow");
    }

    #[test]
    fn mixed_update_array_emits_every_entry_with_no_drop_and_no_null() {
        let known = PermissionUpdate::AddRules {
            rules: vec![PermissionRuleValue {
                tool_name: "Bash".to_string(),
                rule_content: None,
            }],
            behavior: PermissionBehavior::Allow,
            destination: PermissionUpdateDestination::Session,
        };
        let unknown = PermissionUpdate::Unknown(serde_json::json!({"type": "someFutureUpdate"}));
        let decision = PermissionDecision::Allow {
            updated_input: None,
            updated_permissions: vec![known, unknown],
        };
        let value = decision
            .into_response_value(&serde_json::json!({ "cmd": "ls" }))
            .expect("serialize");
        let updates = value["updatedPermissions"]
            .as_array()
            .expect("updatedPermissions must be an array, not null");
        assert_eq!(updates.len(), 2, "no entry may be dropped: {updates:?}");
        assert_eq!(updates[0]["type"], "addRules");
        assert_eq!(updates[1]["type"], "someFutureUpdate");
    }
}
