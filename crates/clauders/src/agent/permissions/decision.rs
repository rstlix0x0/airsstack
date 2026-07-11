//! The tool-permission request context and a policy's verdict.

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

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::PermissionDecision;

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
