//! The rule-update cluster carried on a permission decision and on the
//! binary's own suggestions.

use serde::{Deserialize, Serialize};

use crate::agent::permissions::PermissionMode;

/// One permission-rule update.
///
/// Travels in both directions: the binary sends these as
/// `permission_suggestions` on a `can_use_tool` request, and a decision
/// returns them as `updatedPermissions`.
///
/// Derives `PartialEq` but not `Eq`: `serde_json::Value` does not implement
/// `Eq`, and [`PermissionUpdate::Unknown`] carries one.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PermissionUpdate {
    /// Add the given rules.
    AddRules {
        /// Rules to add.
        rules: Vec<PermissionRuleValue>,
        /// Whether the rules allow, deny, or force an ask.
        behavior: PermissionBehavior,
        /// Where the rules are written.
        destination: PermissionUpdateDestination,
    },
    /// Replace the destination's rules with the given ones.
    ReplaceRules {
        /// Replacement rules.
        rules: Vec<PermissionRuleValue>,
        /// Whether the rules allow, deny, or force an ask.
        behavior: PermissionBehavior,
        /// Where the rules are written.
        destination: PermissionUpdateDestination,
    },
    /// Remove the given rules.
    RemoveRules {
        /// Rules to remove.
        rules: Vec<PermissionRuleValue>,
        /// Which rule list to remove them from.
        behavior: PermissionBehavior,
        /// Where the rules are written.
        destination: PermissionUpdateDestination,
    },
    /// Switch the session's permission mode.
    SetMode {
        /// The mode to apply.
        mode: PermissionMode,
        /// Where the mode is written.
        destination: PermissionUpdateDestination,
    },
    /// Grant access to directories.
    AddDirectories {
        /// Directory paths to add.
        directories: Vec<String>,
        /// Where the grant is written.
        destination: PermissionUpdateDestination,
    },
    /// Revoke access to directories.
    RemoveDirectories {
        /// Directory paths to remove.
        directories: Vec<String>,
        /// Where the revocation is written.
        destination: PermissionUpdateDestination,
    },
    /// A `permission_suggestions` entry whose `type` this release does not
    /// model.
    ///
    /// The binary's `can_use_tool` request carries these as advisory
    /// suggestions a policy may or may not apply; rejecting the whole
    /// request over one unrecognized suggestion would deny a tool call for a
    /// reason unrelated to the tool itself. The raw JSON object is retained
    /// so the surrounding `can_use_tool` body stays decodable.
    ///
    /// Round-trips byte-exactly: a policy that echoes a suggestion back
    /// unchanged (the documented `updated_permissions: ctx.suggestions.clone()`
    /// pattern) must not silently lose it, so serializing this arm emits the
    /// captured object back verbatim rather than erroring.
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

/// A single rule: the tool it governs and an optional argument pattern.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRuleValue {
    /// The tool the rule applies to.
    pub tool_name: String,
    /// The argument pattern the rule matches, when it is narrower than the
    /// whole tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

/// What a rule does when it matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    /// Allow the call without prompting.
    Allow,
    /// Deny the call without prompting.
    Deny,
    /// Force a prompt even when another rule would allow it.
    Ask,
}

/// Where a permission update is persisted.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    /// The user's global settings.
    UserSettings,
    /// The project's shared settings.
    ProjectSettings,
    /// The project's local settings.
    LocalSettings,
    /// The current session only.
    Session,
    /// A command-line argument for this invocation.
    CliArg,
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::{
        PermissionBehavior, PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination,
    };
    use crate::agent::permissions::PermissionMode;

    #[test]
    fn add_rules_round_trips_the_official_shape() {
        let json = r#"{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"ls:*"}],"behavior":"allow","destination":"session"}"#;
        let update: PermissionUpdate = serde_json::from_str(json).expect("deserialize");
        let PermissionUpdate::AddRules {
            rules,
            behavior,
            destination,
        } = &update
        else {
            panic!("expected AddRules, got {update:?}");
        };
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].tool_name, "Bash");
        assert_eq!(rules[0].rule_content.as_deref(), Some("ls:*"));
        assert_eq!(*behavior, PermissionBehavior::Allow);
        assert_eq!(*destination, PermissionUpdateDestination::Session);

        let back = serde_json::to_value(&update).expect("serialize");
        assert_eq!(back["type"], "addRules");
        assert_eq!(back["rules"][0]["toolName"], "Bash");
        assert_eq!(back["destination"], "session");
    }

    #[test]
    fn ask_behavior_round_trips() {
        // 'ask' is the one behavior value the crate has never encoded.
        let json = r#"{"type":"replaceRules","rules":[{"toolName":"Write"}],"behavior":"ask","destination":"projectSettings"}"#;
        let update: PermissionUpdate = serde_json::from_str(json).expect("deserialize");
        let PermissionUpdate::ReplaceRules { behavior, .. } = &update else {
            panic!("expected ReplaceRules");
        };
        assert_eq!(*behavior, PermissionBehavior::Ask);
        let back = serde_json::to_value(&update).expect("serialize");
        assert_eq!(back["behavior"], "ask");
    }

    #[test]
    fn remove_rules_round_trips() {
        let json = r#"{"type":"removeRules","rules":[{"toolName":"Read"}],"behavior":"deny","destination":"userSettings"}"#;
        let update: PermissionUpdate = serde_json::from_str(json).expect("deserialize");
        assert!(matches!(update, PermissionUpdate::RemoveRules { .. }));
        assert_eq!(
            serde_json::to_value(&update).expect("serialize")["type"],
            "removeRules"
        );
    }

    #[test]
    fn set_mode_round_trips() {
        let json = r#"{"type":"setMode","mode":"acceptEdits","destination":"localSettings"}"#;
        let update: PermissionUpdate = serde_json::from_str(json).expect("deserialize");
        let PermissionUpdate::SetMode { mode, destination } = &update else {
            panic!("expected SetMode");
        };
        assert_eq!(*mode, PermissionMode::AcceptEdits);
        assert_eq!(*destination, PermissionUpdateDestination::LocalSettings);
        assert_eq!(
            serde_json::to_value(&update).expect("serialize")["mode"],
            "acceptEdits"
        );
    }

    #[test]
    fn add_directories_round_trips() {
        let json = r#"{"type":"addDirectories","directories":["/tmp/a"],"destination":"cliArg"}"#;
        let update: PermissionUpdate = serde_json::from_str(json).expect("deserialize");
        let PermissionUpdate::AddDirectories {
            directories,
            destination,
        } = &update
        else {
            panic!("expected AddDirectories");
        };
        assert_eq!(directories, &vec!["/tmp/a".to_string()]);
        assert_eq!(*destination, PermissionUpdateDestination::CliArg);
    }

    #[test]
    fn remove_directories_round_trips() {
        let json =
            r#"{"type":"removeDirectories","directories":["/tmp/a"],"destination":"session"}"#;
        let update: PermissionUpdate = serde_json::from_str(json).expect("deserialize");
        assert!(matches!(update, PermissionUpdate::RemoveDirectories { .. }));
    }

    #[test]
    fn unknown_update_type_is_captured_verbatim() {
        let json = r#"{"type":"someFutureUpdate","rules":[{"toolName":"Bash"}]}"#;
        let update: PermissionUpdate = serde_json::from_str(json).expect("deserialize");
        let PermissionUpdate::Unknown(value) = update else {
            panic!("expected Unknown, got {update:?}");
        };
        assert_eq!(value["type"], "someFutureUpdate");
        assert_eq!(value["rules"][0]["toolName"], "Bash");
    }

    #[test]
    fn unknown_update_round_trips_byte_exactly() {
        let update = PermissionUpdate::Unknown(serde_json::json!({"type":"someFutureUpdate"}));
        let wire = serde_json::to_string(&update).expect("serialize");
        assert_eq!(wire, r#"{"type":"someFutureUpdate"}"#);
    }

    #[test]
    fn rule_content_is_omitted_when_absent() {
        let update = PermissionUpdate::AddRules {
            rules: vec![PermissionRuleValue {
                tool_name: "Bash".to_string(),
                rule_content: None,
            }],
            behavior: PermissionBehavior::Allow,
            destination: PermissionUpdateDestination::Session,
        };
        let value = serde_json::to_value(&update).expect("serialize");
        assert!(value["rules"][0].get("ruleContent").is_none());
    }
}
