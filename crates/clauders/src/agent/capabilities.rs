//! The capability manifest carried on the `system`/`init` message frame, and
//! the hook-event wire names.
//!
//! Two independent concerns share this module. [`Capabilities`] is the open
//! flag set the binary advertises; [`HookEvent`] names the events a hook can
//! register for. They are no longer related — the manifest once gated hook
//! registration, and does not any more.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// A hook event the binary may invoke during a turn.
///
/// Wire names are `PascalCase` as the binary emits them. This enum exists so
/// hook registration (see [`crate::agent::hooks::HookRegistry`]) can name the
/// event a hook fires on.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Before a tool is used.
    PreToolUse,
    /// After a tool is used.
    PostToolUse,
    /// After a tool use fails.
    PostToolUseFailure,
    /// When the user submits a prompt.
    UserPromptSubmit,
    /// When the turn stops.
    Stop,
    /// When a subagent starts.
    SubagentStart,
    /// When a subagent stops.
    SubagentStop,
    /// Before context compaction.
    PreCompact,
    /// On a binary notification.
    Notification,
    /// On a permission request.
    PermissionRequest,
    /// When a session starts.
    SessionStart,
    /// When a session ends.
    SessionEnd,
    /// During session setup.
    Setup,
    /// When an MCP server requests user input.
    Elicitation,
    /// After the user responds to an MCP elicitation.
    ElicitationResult,
}

/// Protocol capabilities the binary advertises on its `system`/`init` frame.
///
/// An **open set**: an unrecognized flag is not an error, and an absent flag
/// means the behavior is unavailable on this binary. Older binaries omit the
/// field entirely, which reads as an empty manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    capabilities: HashSet<String>,
}

impl Capabilities {
    /// Whether the binary advertises `flag`.
    #[must_use]
    pub fn supports(&self, flag: &str) -> bool {
        self.capabilities.contains(flag)
    }
}

impl FromIterator<String> for Capabilities {
    /// Build a manifest from flag names, e.g. for test fixtures such as
    /// `MockRuntime::with_capabilities`. The field stays private; this is the
    /// public construction path in place of an inserter or setter.
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Self {
            capabilities: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{Capabilities, HookEvent};

    #[test]
    fn hook_event_round_trips_wire_name() {
        let json = serde_json::to_string(&HookEvent::PreToolUse).expect("serialize");
        assert_eq!(json, "\"PreToolUse\"");
        let back: HookEvent = serde_json::from_str("\"Stop\"").expect("deserialize");
        assert_eq!(back, HookEvent::Stop);
    }

    #[test]
    fn session_end_round_trips_wire_name() {
        let json = serde_json::to_string(&HookEvent::SessionEnd).expect("serialize");
        assert_eq!(json, "\"SessionEnd\"");
        let back: HookEvent = serde_json::from_str("\"SessionEnd\"").expect("deserialize");
        assert_eq!(back, HookEvent::SessionEnd);
    }

    #[test]
    fn setup_round_trips_wire_name() {
        let json = serde_json::to_string(&HookEvent::Setup).expect("serialize");
        assert_eq!(json, "\"Setup\"");
        let back: HookEvent = serde_json::from_str("\"Setup\"").expect("deserialize");
        assert_eq!(back, HookEvent::Setup);
    }

    #[test]
    fn elicitation_round_trips_wire_name() {
        let json = serde_json::to_string(&HookEvent::Elicitation).expect("serialize");
        assert_eq!(json, "\"Elicitation\"");
        let back: HookEvent = serde_json::from_str("\"Elicitation\"").expect("deserialize");
        assert_eq!(back, HookEvent::Elicitation);
    }

    #[test]
    fn elicitation_result_round_trips_wire_name() {
        let json = serde_json::to_string(&HookEvent::ElicitationResult).expect("serialize");
        assert_eq!(json, "\"ElicitationResult\"");
        let back: HookEvent = serde_json::from_str("\"ElicitationResult\"").expect("deserialize");
        assert_eq!(back, HookEvent::ElicitationResult);
    }

    #[test]
    fn parses_the_flag_list_from_an_init_frame() {
        // Verbatim shape of the system/init frame's capabilities field,
        // sdk.d.ts:4405-4407; values from a live probe of binary 2.1.216.
        let json = r#"{"capabilities":["interrupt_receipt_v1","msg_lifecycle_v1"]}"#;
        let caps: Capabilities = serde_json::from_str(json).expect("deserialize");
        assert!(caps.supports("interrupt_receipt_v1"));
        assert!(caps.supports("msg_lifecycle_v1"));
        assert!(!caps.supports("not_a_real_flag"));
    }

    #[test]
    fn absent_flag_list_is_empty_not_an_error() {
        // The field is optional (`capabilities?: string[]`), and older CLIs
        // omit it entirely.
        let caps: Capabilities = serde_json::from_str("{}").expect("deserialize");
        assert!(!caps.supports("interrupt_receipt_v1"));
    }

    #[test]
    fn from_iter_builds_a_manifest_supports_agrees_with() {
        let caps = Capabilities::from_iter([
            "interrupt_receipt_v1".to_string(),
            "msg_lifecycle_v1".to_string(),
        ]);
        assert!(caps.supports("interrupt_receipt_v1"));
        assert!(caps.supports("msg_lifecycle_v1"));
        assert!(!caps.supports("not_a_real_flag"));
    }
}
