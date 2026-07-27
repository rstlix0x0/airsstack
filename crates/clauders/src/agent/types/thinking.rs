//! Extended-thinking configuration (`thinking` startup option).

/// How extended thinking is configured for the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThinkingMode {
    /// Extended thinking on. `budget_tokens: Some(n)` lowers to
    /// `--max-thinking-tokens n`; `None` lowers to `--thinking adaptive`
    /// (the SDK treats enabled-without-budget as adaptive).
    Enabled {
        /// Token budget for thinking, or `None` for adaptive.
        budget_tokens: Option<u32>,
    },
    /// `--thinking disabled`.
    Disabled,
    /// `--thinking adaptive`.
    Adaptive,
}

/// How thinking content appears in the response (`--thinking-display`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThinkingDisplay {
    /// Summarized thinking (`summarized`).
    Summarized,
    /// Thinking omitted from the response (`omitted`).
    Omitted,
}

impl ThinkingDisplay {
    /// The `--thinking-display` token (`summarized` / `omitted`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Summarized => "summarized",
            Self::Omitted => "omitted",
        }
    }
}

/// Startup extended-thinking configuration. `display` is emitted only when
/// `mode` is not [`ThinkingMode::Disabled`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThinkingConfig {
    /// The thinking mode.
    pub mode: ThinkingMode,
    /// Optional display mode (ignored when `mode` is `Disabled`).
    pub display: Option<ThinkingDisplay>,
}

#[cfg(test)]
mod tests {
    use super::{ThinkingConfig, ThinkingDisplay, ThinkingMode};

    #[test]
    fn display_as_str_covers_both() {
        assert_eq!(ThinkingDisplay::Summarized.as_str(), "summarized");
        assert_eq!(ThinkingDisplay::Omitted.as_str(), "omitted");
    }

    #[test]
    fn config_compares_by_value() {
        let a = ThinkingConfig {
            mode: ThinkingMode::Enabled {
                budget_tokens: Some(2048),
            },
            display: Some(ThinkingDisplay::Summarized),
        };
        assert_eq!(
            a,
            ThinkingConfig {
                mode: ThinkingMode::Enabled {
                    budget_tokens: Some(2048)
                },
                display: Some(ThinkingDisplay::Summarized)
            }
        );
        assert_ne!(a.mode, ThinkingMode::Adaptive);
    }
}
