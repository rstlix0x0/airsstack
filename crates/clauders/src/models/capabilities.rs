//! Model capability-discovery types returned by the Models API.
//!
//! Exists as its own module so the nine-member capability tree is decoupled
//! from the `ModelInfo` record and the resource dispatch logic.

use std::collections::BTreeMap;

/// Whether a single capability is supported by a model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct CapabilitySupport {
    /// Whether this capability is supported.
    #[serde(default)]
    pub supported: bool,
}

/// Effort-level support for a model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct EffortCapability {
    /// Low effort support.
    #[serde(default)]
    pub low: CapabilitySupport,
    /// Medium effort support.
    #[serde(default)]
    pub medium: CapabilitySupport,
    /// High effort support.
    #[serde(default)]
    pub high: CapabilitySupport,
    /// Max effort support.
    #[serde(default)]
    pub max: CapabilitySupport,
    /// Extra-high effort support, when the model reports it.
    #[serde(default)]
    pub xhigh: Option<CapabilitySupport>,
    /// Whether effort selection is supported at all.
    #[serde(default)]
    pub supported: bool,
}

/// Context-management support and available dated strategies.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct ContextManagementCapability {
    /// Whether context management is supported at all.
    #[serde(default)]
    pub supported: bool,
    /// Dated strategy toggles (e.g. `clear_thinking_20251015`), captured as a
    /// forward-safe map so new dated keys need no code change.
    #[serde(flatten)]
    pub strategies: BTreeMap<String, CapabilitySupport>,
}

/// Supported thinking-type configurations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct ThinkingTypes {
    /// Adaptive (auto) thinking support.
    #[serde(default)]
    pub adaptive: CapabilitySupport,
    /// Explicit `enabled` thinking support.
    #[serde(default)]
    pub enabled: CapabilitySupport,
}

/// Thinking support for a model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct ThinkingCapability {
    /// Whether thinking is supported at all.
    #[serde(default)]
    pub supported: bool,
    /// Which thinking-type configurations are supported.
    #[serde(default)]
    pub types: ThinkingTypes,
}

/// Full capability-discovery record for a model.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct ModelCapabilities {
    /// Batch API support.
    #[serde(default)]
    pub batch: CapabilitySupport,
    /// Citation-generation support.
    #[serde(default)]
    pub citations: CapabilitySupport,
    /// Code-execution tool support.
    #[serde(default)]
    pub code_execution: CapabilitySupport,
    /// Image content-block support.
    #[serde(default)]
    pub image_input: CapabilitySupport,
    /// PDF content-block support.
    #[serde(default)]
    pub pdf_input: CapabilitySupport,
    /// Structured-output / JSON-mode support.
    #[serde(default)]
    pub structured_outputs: CapabilitySupport,
    /// Context-management support and strategies.
    #[serde(default)]
    pub context_management: ContextManagementCapability,
    /// Effort-level support.
    #[serde(default)]
    pub effort: EffortCapability,
    /// Thinking support.
    #[serde(default)]
    pub thinking: ThinkingCapability,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn full_capabilities_decode() {
        let json = r#"{
            "batch":{"supported":true},
            "citations":{"supported":false},
            "code_execution":{"supported":true},
            "image_input":{"supported":true},
            "pdf_input":{"supported":true},
            "structured_outputs":{"supported":true},
            "context_management":{"supported":true,
                "clear_thinking_20251015":{"supported":true},
                "compact_20260112":{"supported":false}},
            "effort":{"low":{"supported":true},"medium":{"supported":true},
                "high":{"supported":true},"max":{"supported":true},
                "xhigh":{"supported":false},"supported":true},
            "thinking":{"supported":true,
                "types":{"adaptive":{"supported":true},"enabled":{"supported":true}}}
        }"#;
        let c: ModelCapabilities = serde_json::from_str(json).unwrap();
        assert!(c.batch.supported);
        assert!(!c.citations.supported);
        assert_eq!(c.effort.xhigh, Some(CapabilitySupport { supported: false }));
        assert!(c.thinking.types.adaptive.supported);
    }

    #[test]
    fn context_management_dated_keys_land_in_the_map() {
        let json = r#"{"supported":true,
            "clear_thinking_20251015":{"supported":true},
            "clear_tool_uses_20250919":{"supported":false}}"#;
        let cm: ContextManagementCapability = serde_json::from_str(json).unwrap();
        assert_eq!(cm.strategies.len(), 2);
        assert_eq!(
            cm.strategies.get("clear_thinking_20251015"),
            Some(&CapabilitySupport { supported: true })
        );
    }

    #[test]
    fn missing_capability_booleans_default_to_false() {
        let c: ModelCapabilities = serde_json::from_str("{}").unwrap();
        assert!(!c.batch.supported);
        assert!(!c.effort.supported);
        assert_eq!(c.effort.xhigh, None);
        assert!(c.context_management.strategies.is_empty());
    }
}
