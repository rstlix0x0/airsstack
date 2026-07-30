//! Reasoning-effort level shared by the Messages API and the Agent SDK.

use serde::Serialize;

/// How much reasoning effort a model should spend.
///
/// The same five levels serve both pillars of this crate:
///
/// - the Messages API sends the value as `output_config.effort`, using the
///   lowercase form produced by [`Serialize`];
/// - the Agent SDK lowers it to the `claude` CLI's `--effort <level>` flag
///   via [`EffortLevel::as_str`].
///
/// Both forms are the same five lowercase strings, so the two pillars cannot
/// drift apart.
///
/// # Examples
///
/// ```
/// use clauders::types::EffortLevel;
///
/// assert_eq!(EffortLevel::Xhigh.as_str(), "xhigh");
/// assert_eq!(
///     serde_json::to_value(EffortLevel::Xhigh).unwrap(),
///     serde_json::json!("xhigh")
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EffortLevel {
    /// Least reasoning effort.
    Low,
    /// Moderate reasoning effort.
    Medium,
    /// Elevated reasoning effort.
    High,
    /// Extra-high reasoning effort.
    Xhigh,
    /// Maximum reasoning effort.
    Max,
}

impl EffortLevel {
    /// This level as its wire string
    /// (`low` / `medium` / `high` / `xhigh` / `max`).
    ///
    /// Identical to the value [`Serialize`] produces, so it is equally the
    /// `output_config.effort` JSON value and the `--effort` CLI argument.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test asserts known-valid serialization")]

    use super::EffortLevel;

    #[test]
    fn as_str_covers_every_level() {
        assert_eq!(EffortLevel::Low.as_str(), "low");
        assert_eq!(EffortLevel::Medium.as_str(), "medium");
        assert_eq!(EffortLevel::High.as_str(), "high");
        assert_eq!(EffortLevel::Xhigh.as_str(), "xhigh");
        assert_eq!(EffortLevel::Max.as_str(), "max");
    }

    #[test]
    fn serialize_matches_as_str_for_every_level() {
        for level in [
            EffortLevel::Low,
            EffortLevel::Medium,
            EffortLevel::High,
            EffortLevel::Xhigh,
            EffortLevel::Max,
        ] {
            let serialized = serde_json::to_value(level).expect("serialize");
            assert_eq!(serialized, serde_json::json!(level.as_str()));
        }
    }

    /// Both pillars name one type, so their wire forms cannot drift apart.
    #[test]
    fn effort_level_is_reachable_from_both_pillars() {
        let from_types = crate::types::EffortLevel::Xhigh;
        let from_agent = crate::agent::EffortLevel::Xhigh;
        assert_eq!(from_types, from_agent);
        assert_eq!(
            serde_json::to_value(from_types).expect("serialize"),
            serde_json::json!("xhigh")
        );
    }
}
