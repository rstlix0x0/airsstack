//! The hook events the harness understands.
//!
//! Responsibilities: [`HookEvent`] and [`InvalidHookEvent`]. The variants are
//! the events observed in real hooks.json files in this repository; the set is
//! `#[non_exhaustive]` at the parse level (an unknown event is an error naming
//! the known ones) and grows as capture (P4) grounds more.

/// A hook event name from hooks.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HookEvent {
    /// Before a tool call runs. Exit 2 blocks the call.
    PreToolUse,
    /// After a tool call ran.
    PostToolUse,
    /// When the user submits a prompt.
    UserPromptSubmit,
    /// At session start/resume/clear/compact.
    SessionStart,
    /// At session end.
    SessionEnd,
}

/// Why a string is not a known hook event.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error(
    "unknown hook event `{0}` (known: PreToolUse, PostToolUse, UserPromptSubmit, SessionStart, SessionEnd)"
)]
pub struct InvalidHookEvent(String);

impl core::str::FromStr for HookEvent {
    type Err = InvalidHookEvent;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "PreToolUse" => Ok(Self::PreToolUse),
            "PostToolUse" => Ok(Self::PostToolUse),
            "UserPromptSubmit" => Ok(Self::UserPromptSubmit),
            "SessionStart" => Ok(Self::SessionStart),
            "SessionEnd" => Ok(Self::SessionEnd),
            other => Err(InvalidHookEvent(other.to_owned())),
        }
    }
}

impl HookEvent {
    /// The event name as it appears in hooks.json.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::HookEvent;

    #[test]
    fn every_variant_round_trips_through_its_name() {
        for event in [
            HookEvent::PreToolUse,
            HookEvent::PostToolUse,
            HookEvent::UserPromptSubmit,
            HookEvent::SessionStart,
            HookEvent::SessionEnd,
        ] {
            assert_eq!(event.as_str().parse::<HookEvent>(), Ok(event));
        }
    }

    #[test]
    fn an_unknown_event_is_an_error_naming_the_known_set() {
        let error = "Frobnicate".parse::<HookEvent>().unwrap_err();
        assert!(error.to_string().contains("PreToolUse"));
    }
}
