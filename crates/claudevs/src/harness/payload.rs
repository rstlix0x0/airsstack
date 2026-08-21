//! Hook payloads: built-in defaults, case overlay, `{project}` substitution.
//!
//! The defaults are provisional: hand-written against the shapes
//! this repository's own hooks parse today, replaced as a base layer by
//! captured payloads once capture exists. The overlay mechanism is the same
//! either way, so swapping the base touches no case.

use crate::types::HookEvent;

/// The built-in default payload for `event`.
#[must_use]
pub fn default_payload(event: HookEvent) -> serde_json::Value {
    let base = serde_json::json!({
        "session_id": "claudevs-test",
        "cwd": "{project}",
        "hook_event_name": event.as_str(),
    });
    let mut value = base;
    let extra = match event {
        HookEvent::PreToolUse | HookEvent::PostToolUse => serde_json::json!({
            "tool_name": "Edit",
            "tool_input": { "file_path": "{project}/file.txt" },
        }),
        HookEvent::UserPromptSubmit => serde_json::json!({ "prompt": "hello" }),
        HookEvent::SessionStart => serde_json::json!({ "source": "startup" }),
        HookEvent::SessionEnd => serde_json::json!({ "reason": "exit" }),
    };
    merge(&mut value, &extra);
    value
}

/// Overlays `over` onto `base`: objects merge recursively, everything else replaces.
pub fn merge(base: &mut serde_json::Value, over: &serde_json::Value) {
    match (base, over) {
        (serde_json::Value::Object(b), serde_json::Value::Object(o)) => {
            for (key, value) in o {
                merge(
                    b.entry(key.clone()).or_insert(serde_json::Value::Null),
                    value,
                );
            }
        }
        (slot, other) => *slot = other.clone(),
    }
}

/// Replaces `{project}` in every string of `value` with `project`.
#[expect(
    clippy::literal_string_with_formatting_args,
    reason = "the `{project}` literal is a placeholder token replaced by str::replace, not a format string"
)]
pub fn substitute_project(value: &mut serde_json::Value, project: &str) {
    match value {
        serde_json::Value::String(s) => {
            if s.contains("{project}") {
                *s = s.replace("{project}", project);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                substitute_project(item, project);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                substitute_project(item, project);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{default_payload, merge, substitute_project};
    use crate::types::HookEvent;

    #[test]
    fn an_overlaid_field_wins_and_siblings_survive() {
        let mut payload = default_payload(HookEvent::PreToolUse);
        merge(
            &mut payload,
            &serde_json::json!({ "tool_input": { "file_path": "Cargo.lock" } }),
        );
        assert_eq!(payload["tool_input"]["file_path"], "Cargo.lock");
        assert_eq!(payload["tool_name"], "Edit"); // sibling default kept
        assert_eq!(payload["hook_event_name"], "PreToolUse");
    }

    #[test]
    fn project_placeholders_resolve_everywhere_in_the_tree() {
        let mut payload = default_payload(HookEvent::PreToolUse);
        substitute_project(&mut payload, "/tmp/p1");
        assert_eq!(payload["cwd"], "/tmp/p1");
        assert_eq!(payload["tool_input"]["file_path"], "/tmp/p1/file.txt");
    }

    #[test]
    fn every_event_has_a_default_payload_object() {
        for event in [
            HookEvent::PreToolUse,
            HookEvent::PostToolUse,
            HookEvent::UserPromptSubmit,
            HookEvent::SessionStart,
            HookEvent::SessionEnd,
        ] {
            assert!(default_payload(event).is_object(), "{event:?}");
        }
    }
}
