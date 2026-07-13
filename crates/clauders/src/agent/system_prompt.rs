//! User-facing system-prompt configuration for the agent layer.
//!
//! Distinct from the wire-level [`crate::types::SystemPrompt`]: this expresses
//! caller intent (including the `claude_code` preset), which each runtime lowers
//! to its own representation at request-build time.

/// System-prompt configuration carried on [`crate::agent::options::Options`].
///
/// Lowered per-runtime at request build: `CliRuntime` maps it to argv flags,
/// native runtimes call [`SystemPromptConfig::native_text`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SystemPromptConfig {
    /// No system prompt.
    #[default]
    None,
    /// A verbatim system prompt string.
    Text(String),
    /// The built-in `claude_code` preset, optionally appended to.
    Preset {
        /// Extra instructions appended after the preset base.
        append: Option<String>,
        /// Move per-session dynamic sections (cwd, git status, …) to the
        /// first user message rather than the system prompt.
        exclude_dynamic_sections: bool,
    },
}

impl SystemPromptConfig {
    /// The plain text a native runtime should send as its system prompt.
    ///
    /// A `Preset` degrades to its `append`: the `claude_code` base prompt is
    /// unavailable off the CLI. `None` and a base-less preset both yield
    /// `None` (no system prompt at all).
    #[must_use]
    pub fn native_text(&self) -> Option<String> {
        match self {
            Self::None => Option::None,
            Self::Text(s) => Some(s.clone()),
            Self::Preset { append, .. } => append.clone(),
        }
    }

    /// Whether this is the `claude_code` preset, whose base is dropped on a
    /// native runtime.
    #[must_use]
    pub const fn is_preset(&self) -> bool {
        matches!(self, Self::Preset { .. })
    }
}

impl From<String> for SystemPromptConfig {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for SystemPromptConfig {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::SystemPromptConfig;

    #[test]
    fn default_is_none() {
        assert_eq!(SystemPromptConfig::default(), SystemPromptConfig::None);
    }

    #[test]
    fn from_str_and_string_make_text() {
        assert_eq!(
            SystemPromptConfig::from("hi"),
            SystemPromptConfig::Text("hi".to_owned())
        );
        assert_eq!(
            SystemPromptConfig::from("hi".to_owned()),
            SystemPromptConfig::Text("hi".to_owned())
        );
    }

    #[test]
    fn native_text_none_yields_none() {
        assert_eq!(SystemPromptConfig::None.native_text(), None);
    }

    #[test]
    fn native_text_text_yields_the_string() {
        assert_eq!(
            SystemPromptConfig::Text("be terse".to_owned()).native_text(),
            Some("be terse".to_owned())
        );
    }

    #[test]
    fn native_text_preset_degrades_to_append() {
        let with = SystemPromptConfig::Preset {
            append: Some("extra".to_owned()),
            exclude_dynamic_sections: false,
        };
        assert_eq!(with.native_text(), Some("extra".to_owned()));

        let without = SystemPromptConfig::Preset {
            append: None,
            exclude_dynamic_sections: true,
        };
        assert_eq!(without.native_text(), None);
    }

    #[test]
    fn is_preset_only_true_for_preset() {
        assert!(!SystemPromptConfig::None.is_preset());
        assert!(!SystemPromptConfig::Text("x".to_owned()).is_preset());
        assert!(
            SystemPromptConfig::Preset {
                append: None,
                exclude_dynamic_sections: false
            }
            .is_preset()
        );
    }
}
