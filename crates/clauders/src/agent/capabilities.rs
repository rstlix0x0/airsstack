//! Capability manifest negotiated during the initialize handshake.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// A hook event the binary may invoke during a turn.
///
/// Wire names are `PascalCase` as the binary emits them. This enum exists so
/// [`Capabilities`] can record which events a given binary supports.
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
}

/// Features advertised by the binary in its initialize-handshake response.
///
/// Used to gate optional features and degrade gracefully across binary
/// versions: a feature absent from the manifest is treated as unsupported
/// rather than assumed present.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Control-protocol version string.
    #[serde(default)]
    pub protocol_version: String,
    /// Hook events the binary will invoke.
    #[serde(default)]
    pub supported_hook_events: HashSet<HookEvent>,
    /// Control methods the binary accepts.
    #[serde(default)]
    pub supported_control_methods: HashSet<String>,
}

impl Capabilities {
    /// Whether the binary supports `event`.
    #[must_use]
    pub fn supports_hook(&self, event: HookEvent) -> bool {
        self.supported_hook_events.contains(&event)
    }

    /// Whether the binary accepts the control `method`.
    #[must_use]
    pub fn supports_control(&self, method: &str) -> bool {
        self.supported_control_methods.contains(method)
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
    fn parses_capabilities_from_handshake() {
        let json = r#"{
            "protocol_version": "1.0",
            "supported_hook_events": ["PreToolUse", "Stop"],
            "supported_control_methods": ["interrupt", "set_model"]
        }"#;
        let caps: Capabilities = serde_json::from_str(json).expect("deserialize");
        assert_eq!(caps.protocol_version, "1.0");
        assert!(caps.supports_hook(HookEvent::PreToolUse));
        assert!(!caps.supports_hook(HookEvent::Notification));
        assert!(caps.supports_control("interrupt"));
        assert!(!caps.supports_control("teleport"));
    }

    #[test]
    fn missing_fields_default_to_empty() {
        let caps: Capabilities = serde_json::from_str("{}").expect("deserialize");
        assert!(caps.protocol_version.is_empty());
        assert!(!caps.supports_hook(HookEvent::Stop));
    }
}
