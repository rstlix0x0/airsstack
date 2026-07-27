//! Sandbox configuration (`sandbox` startup option), folded into `--settings`.

use serde::{Deserialize, Serialize};

/// Sandbox configuration folded into the binary's `--settings` payload under a
/// `sandbox` key.
///
/// Only `enabled` and `fail_if_unavailable` — the two fields the official
/// SDK's argv builder special-cases — are typed; the rest of the
/// (version-drifting) sandbox schema passes through the flattened tail unchanged.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
// `serde_json::Value` in the flattened tail does not implement `Eq`.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "the flattened `extra` tail holds serde_json::Value, which is not Eq"
)]
pub struct SandboxConfig {
    /// Whether the sandbox is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Exit with an error at startup if the sandbox is enabled but unavailable.
    #[serde(rename = "failIfUnavailable", skip_serializing_if = "Option::is_none")]
    pub fail_if_unavailable: Option<bool>,
    /// The remainder of the sandbox schema (network, filesystem, credentials,
    /// deny lists, …), passed through untyped.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test asserts known-valid serialization")]

    use super::SandboxConfig;

    #[test]
    fn serializes_known_fields_with_camelcase_rename() {
        let cfg = SandboxConfig {
            enabled: Some(true),
            fail_if_unavailable: Some(true),
            ..Default::default()
        };
        let v = serde_json::to_value(&cfg).expect("serialize");
        assert_eq!(
            v,
            serde_json::json!({ "enabled": true, "failIfUnavailable": true })
        );
    }

    #[test]
    fn omits_unset_known_fields() {
        let v = serde_json::to_value(SandboxConfig::default()).expect("serialize");
        assert_eq!(v, serde_json::json!({}));
    }

    #[test]
    fn preserves_the_flattened_tail_round_trip() {
        let src = serde_json::json!({
            "enabled": true,
            "network": { "allowManagedDomainsOnly": true },
            "allowUnsandboxedCommands": false
        });
        let cfg: SandboxConfig = serde_json::from_value(src.clone()).expect("deserialize");
        assert_eq!(cfg.enabled, Some(true));
        assert!(cfg.extra.contains_key("network"));
        assert_eq!(serde_json::to_value(&cfg).expect("serialize"), src);
    }
}
