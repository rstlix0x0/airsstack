//! Permission control for the agent.
//!
//! Defines the [`PermissionMode`] data enum forwarded to the binary on the
//! `set_permission_mode` control request and carried in [`crate::agent::Options`].

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::PermissionMode;

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
}
