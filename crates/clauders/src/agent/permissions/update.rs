//! The rule-update cluster returned by a decision's `updated_permissions`.

use serde::{Deserialize, Serialize};

/// A single permission-rule update returned by a [`super::PermissionDecision`].
///
/// Serialized into the CLI `updatedPermissions` array for passthrough. Natively,
/// only `tool` + `behavior` are acted on; `scope` is carried for CLI fidelity but
/// the native rule store treats every update as session-scoped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionUpdate {
    /// Whether the rule allows or denies the named tool.
    pub behavior: PermissionBehavior,
    /// The tool name the rule applies to.
    pub tool: String,
    /// Where the rule is meant to persist.
    pub scope: PermissionScope,
}

/// A bare allow/deny discriminant shared by [`PermissionUpdate`] and the native
/// rule store. Distinct in shape and role from the payload-carrying
/// [`super::PermissionDecision`] — not a duplicate of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    /// Allow the named tool.
    Allow,
    /// Deny the named tool.
    Deny,
}

/// Where a permission rule is meant to persist.
///
/// Carried for CLI wire fidelity; the native store honors session semantics only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionScope {
    /// The current session only.
    Session,
    /// The current project's local settings.
    Local,
    /// The current project's shared settings.
    Project,
    /// The user's global settings.
    User,
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{PermissionBehavior, PermissionScope, PermissionUpdate};

    #[test]
    fn update_serializes_to_wire_shape() {
        let update = PermissionUpdate {
            behavior: PermissionBehavior::Allow,
            tool: "Bash".to_string(),
            scope: PermissionScope::Session,
        };
        let value = serde_json::to_value(&update).expect("serialize");
        assert_eq!(value["behavior"], "allow");
        assert_eq!(value["tool"], "Bash");
        assert_eq!(value["scope"], "session");
    }

    #[test]
    fn behavior_serializes_lowercase() {
        assert_eq!(
            serde_json::to_value(PermissionBehavior::Deny).expect("serialize"),
            serde_json::json!("deny")
        );
    }

    #[test]
    fn scope_serializes_lowercase() {
        for (scope, wire) in [
            (PermissionScope::Session, "session"),
            (PermissionScope::Local, "local"),
            (PermissionScope::Project, "project"),
            (PermissionScope::User, "user"),
        ] {
            assert_eq!(
                serde_json::to_value(scope).expect("serialize"),
                serde_json::json!(wire)
            );
        }
    }
}
