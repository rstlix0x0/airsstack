//! The tool-permission request context and a policy's verdict.

use super::update::PermissionUpdate;

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
    #[must_use]
    pub fn into_response_value(self, original_input: &serde_json::Value) -> serde_json::Value {
        match self {
            Self::Allow {
                updated_input,
                updated_permissions,
            } => {
                let mut value = serde_json::json!({
                    "behavior": "allow",
                    "updatedInput": updated_input.unwrap_or_else(|| original_input.clone()),
                });
                attach_updates(&mut value, &updated_permissions);
                value
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
                attach_updates(&mut value, &updated_permissions);
                value
            }
        }
    }
}

/// Attach a non-empty `updatedPermissions` array to a response value.
fn attach_updates(value: &mut serde_json::Value, updates: &[PermissionUpdate]) {
    if !updates.is_empty() {
        value["updatedPermissions"] = serde_json::to_value(updates).unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::PermissionDecision;
    use super::PermissionUpdate;
    use crate::agent::permissions::{PermissionBehavior, PermissionScope};

    #[test]
    fn allow_without_rewrite_echoes_original_input() {
        let original = serde_json::json!({ "cmd": "ls" });
        let value = PermissionDecision::allow().into_response_value(&original);
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["updatedInput"], serde_json::json!({ "cmd": "ls" }));
    }

    #[test]
    fn allow_with_rewrite_uses_updated_input() {
        let original = serde_json::json!({ "cmd": "ls" });
        let rewritten = serde_json::json!({ "cmd": "ls -la" });
        let value =
            PermissionDecision::allow_with(rewritten.clone()).into_response_value(&original);
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["updatedInput"], rewritten);
    }

    #[test]
    fn deny_carries_message() {
        let value = PermissionDecision::deny("blocked by policy")
            .into_response_value(&serde_json::json!({}));
        assert_eq!(value["behavior"], "deny");
        assert_eq!(value["message"], "blocked by policy");
    }

    #[test]
    fn deny_constructor_defaults_interrupt_false_no_updates() {
        let decision = PermissionDecision::deny("nope");
        assert!(decision.updated_permissions().is_empty());
        let value = decision.into_response_value(&serde_json::json!({}));
        assert_eq!(value["behavior"], "deny");
        assert_eq!(value["message"], "nope");
        assert_eq!(value["interrupt"], false);
        assert!(value.get("updatedPermissions").is_none());
    }

    #[test]
    fn deny_interrupt_constructor_sets_interrupt_true() {
        let value =
            PermissionDecision::deny_interrupt("stop").into_response_value(&serde_json::json!({}));
        assert_eq!(value["interrupt"], true);
    }

    #[test]
    fn allow_constructor_echoes_input_and_omits_empty_updates() {
        let original = serde_json::json!({ "cmd": "ls" });
        let value = PermissionDecision::allow().into_response_value(&original);
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["updatedInput"]["cmd"], "ls");
        assert!(value.get("updatedPermissions").is_none());
    }

    #[test]
    fn updated_permissions_serialize_into_the_response() {
        let update = PermissionUpdate {
            behavior: PermissionBehavior::Allow,
            tool: "Bash".to_string(),
            scope: PermissionScope::Session,
        };
        let decision = PermissionDecision::Deny {
            message: "denied".to_string(),
            interrupt: false,
            updated_permissions: vec![update],
        };
        let value = decision.into_response_value(&serde_json::json!({}));
        assert_eq!(value["updatedPermissions"][0]["tool"], "Bash");
        assert_eq!(value["updatedPermissions"][0]["behavior"], "allow");
    }
}
