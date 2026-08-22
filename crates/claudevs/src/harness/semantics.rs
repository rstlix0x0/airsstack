//! What an observed run *means*, per hook event.
//!
//! The one place the exit-code and envelope knowledge lives: cases
//! state meaning (`decision: deny`) and this module translates observation into
//! it. Grounded rules, each carried by a test:
//!
//! - A JSON envelope on stdout may carry `hookSpecificOutput.permissionDecision`
//!   (one of allow/deny/ask/defer) and `hookSpecificOutput.additionalContext`,
//!   independently of one another — either field may be present without the other.
//! - `PreToolUse` exit 2 blocks the tool call — observed as `Decision::Deny`.
//! - A `SessionStart` hook's bare stdout (no envelope) is injected context.

use crate::case::Decision;
use crate::harness::Captured;
use crate::types::HookEvent;

/// The meaning extracted from one captured run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observed {
    /// Exit code.
    pub exit: i32,
    /// The decision, when one was communicated.
    pub decision: Option<Decision>,
    /// Injected context, when any was communicated.
    pub context: Option<String>,
    /// Whether the hook emitted any envelope or context at all.
    pub emitted: bool,
    /// Whether the child was killed by the case timeout.
    pub timed_out: bool,
    /// Raw stdout.
    pub stdout: String,
    /// Raw stderr.
    pub stderr: String,
}

/// Interprets `captured` under `event`'s semantics.
#[must_use]
pub fn observe(event: HookEvent, captured: &Captured) -> Observed {
    let mut observed = Observed {
        exit: captured.exit,
        stdout: captured.stdout.clone(),
        stderr: captured.stderr.clone(),
        timed_out: captured.timed_out,
        ..Observed::default()
    };

    let envelope: Option<serde_json::Value> = serde_json::from_str(captured.stdout.trim()).ok();
    if let Some(specific) = envelope.as_ref().and_then(|e| e.get("hookSpecificOutput")) {
        observed.emitted = true;
        observed.decision = specific
            .get("permissionDecision")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| match s {
                "allow" => Some(Decision::Allow),
                "deny" => Some(Decision::Deny),
                "ask" => Some(Decision::Ask),
                "defer" => Some(Decision::Defer),
                _ => None,
            });
        observed.context = specific
            .get("additionalContext")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
    } else if event == HookEvent::SessionStart && !captured.stdout.trim().is_empty() {
        observed.emitted = true;
        observed.context = Some(captured.stdout.trim().to_owned());
    }

    if event == HookEvent::PreToolUse && captured.exit == 2 {
        observed.emitted = true;
        observed.decision = Some(Decision::Deny);
    }

    observed
}

#[cfg(test)]
mod tests {
    use super::observe;
    use crate::case::Decision;
    use crate::harness::Captured;
    use crate::types::HookEvent;

    fn captured(exit: i32, stdout: &str, stderr: &str) -> Captured {
        Captured {
            exit,
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
            timed_out: false,
        }
    }

    #[test]
    fn an_envelope_decision_and_context_are_extracted() {
        let json = r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"defer","additionalContext":"read the guideline"}}"#;
        let observed = observe(HookEvent::PreToolUse, &captured(0, json, ""));
        assert_eq!(observed.decision, Some(Decision::Defer));
        assert_eq!(observed.context.as_deref(), Some("read the guideline"));
        assert!(observed.emitted);
    }

    #[test]
    fn an_envelope_decision_allow_is_extracted() {
        let json =
            r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}"#;
        let observed = observe(HookEvent::PreToolUse, &captured(0, json, ""));
        assert_eq!(observed.decision, Some(Decision::Allow));
    }

    #[test]
    fn an_envelope_decision_deny_is_extracted() {
        let json =
            r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny"}}"#;
        let observed = observe(HookEvent::PreToolUse, &captured(0, json, ""));
        assert_eq!(observed.decision, Some(Decision::Deny));
    }

    #[test]
    fn an_envelope_decision_ask_is_extracted() {
        let json =
            r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask"}}"#;
        let observed = observe(HookEvent::PreToolUse, &captured(0, json, ""));
        assert_eq!(observed.decision, Some(Decision::Ask));
    }

    #[test]
    fn an_unrecognised_permission_decision_string_leaves_decision_none() {
        let json =
            r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"maybe"}}"#;
        let observed = observe(HookEvent::PreToolUse, &captured(0, json, ""));
        assert_eq!(observed.decision, None);
        assert!(observed.emitted);
    }

    #[test]
    fn an_envelope_with_context_and_no_decision_leaves_decision_none() {
        let json = r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"rust-guidelines apply"}}"#;
        let observed = observe(HookEvent::PreToolUse, &captured(0, json, ""));
        assert_eq!(observed.decision, None);
        assert_eq!(observed.context.as_deref(), Some("rust-guidelines apply"));
        assert!(observed.emitted);
    }

    #[test]
    fn pretooluse_exit_two_means_deny_even_without_an_envelope() {
        let observed = observe(HookEvent::PreToolUse, &captured(2, "", "blocked"));
        assert_eq!(observed.decision, Some(Decision::Deny));
    }

    #[test]
    fn exit_two_means_nothing_special_on_other_events() {
        let observed = observe(HookEvent::SessionEnd, &captured(2, "", ""));
        assert_eq!(observed.decision, None);
        assert!(!observed.emitted);
    }

    #[test]
    fn sessionstart_bare_stdout_is_context() {
        let observed = observe(HookEvent::SessionStart, &captured(0, "remember X\n", ""));
        assert_eq!(observed.context.as_deref(), Some("remember X"));
    }

    #[test]
    fn bare_stdout_on_pretooluse_is_not_context() {
        let observed = observe(HookEvent::PreToolUse, &captured(0, "chatter\n", ""));
        assert_eq!(observed.context, None);
        assert!(!observed.emitted);
    }

    #[test]
    fn a_timeout_kill_is_propagated_to_the_observation() {
        let mut killed = captured(-2, "", "");
        killed.timed_out = true;
        assert!(observe(HookEvent::PreToolUse, &killed).timed_out);
    }

    #[test]
    fn silence_is_observed_as_no_emission() {
        let observed = observe(HookEvent::PreToolUse, &captured(0, "", ""));
        assert!(!observed.emitted);
    }
}
