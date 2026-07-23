//! The retained `initialize` control-response payload and its accessors.

use serde::{Deserialize, Serialize};

/// The `initialize` control response, retained for zero-wire accessors.
///
/// Fields: `commands`, `agents`, `output_style`, `available_output_styles`,
/// `models`, `account`, `pid`. Element shapes of the list fields are not
/// modeled here (opaque values); `account` is opaque.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "serde_json::Value fields (f64-backed) cannot derive Eq"
)]
pub struct InitializeResult {
    /// Available slash commands.
    #[serde(default)]
    pub commands: Vec<serde_json::Value>,
    /// Available agents.
    #[serde(default)]
    pub agents: Vec<serde_json::Value>,
    /// Available models.
    #[serde(default)]
    pub models: Vec<serde_json::Value>,
    /// Account info block. Opaque.
    #[serde(default)]
    pub account: serde_json::Value,
    /// Any fields this release does not model, retained opaquely.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]
    use super::InitializeResult;

    #[test]
    fn initialize_result_binds_accessor_fields_and_retains_extra() {
        // [binary v2.1.216] verbatim mock-init response shape.
        let json = r#"{"commands":[],"agents":[],"output_style":"normal","available_output_styles":["normal"],"models":[],"account":{},"pid":123}"#;
        let r: InitializeResult = serde_json::from_str(json).expect("deserialize");
        assert!(r.commands.is_empty());
        assert_eq!(r.account, serde_json::json!({}));
        assert_eq!(r.extra.get("pid"), Some(&serde_json::json!(123)));
    }

    #[test]
    fn initialize_result_binds_a_real_live_response() {
        // [probe v2.1.216]: captured verbatim from a live `initialize`
        // control response against the v2.1.216 binary.
        let json = r#"{"commands":[{"name":"deep-research"}],"agents":[{"name":"airsstack-journal:journal-capture"}],"output_style":"default","available_output_styles":["default","explanatory"],"models":[{"value":"default","resolvedModel":"claude-opus-4-8[1m]"}],"account":{"email":"user@example.com","organization":"Example Org","subscriptionType":"Claude Max","apiProvider":"firstParty"},"pid":70515,"remote_control_auto_enable":false,"remote_control_auto_on_by_default":false,"ide_rc_auto_enable_gate":false}"#;
        let r: InitializeResult = serde_json::from_str(json).expect("deserialize");
        assert_eq!(r.commands.len(), 1);
        assert_eq!(r.agents.len(), 1);
        assert_eq!(r.models.len(), 1);
        assert_eq!(r.account["subscriptionType"], "Claude Max");
        assert_eq!(r.extra.get("pid"), Some(&serde_json::json!(70515)));
        assert_eq!(
            r.extra.get("output_style"),
            Some(&serde_json::json!("default"))
        );
        assert_eq!(
            r.extra.get("remote_control_auto_enable"),
            Some(&serde_json::json!(false))
        );
    }
}
