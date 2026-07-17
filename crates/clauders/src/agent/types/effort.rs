//! Reasoning-effort level for a `claude` session.

/// Reasoning-effort level for a `claude` session, lowered to the CLI's
/// `--effort <level>` flag. The five values are the complete set the binary
/// (`v2.1.209`) accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// The CLI flag value for this level
    /// (`low` / `medium` / `high` / `xhigh` / `max`).
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
    use super::EffortLevel;

    #[test]
    fn as_str_covers_every_level() {
        assert_eq!(EffortLevel::Low.as_str(), "low");
        assert_eq!(EffortLevel::Medium.as_str(), "medium");
        assert_eq!(EffortLevel::High.as_str(), "high");
        assert_eq!(EffortLevel::Xhigh.as_str(), "xhigh");
        assert_eq!(EffortLevel::Max.as_str(), "max");
    }
}
