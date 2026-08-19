//! Reading a plugin's hooks/hooks.json and resolving a case's hook reference.
//!
//! The file shape (verified against this repository's plugins):
//! `{"hooks": {"<Event>": [{"matcher": "...", "hooks": [{"type": "command", "command": "..."}]}]}}`.
//!
//! Resolution: no `hook:` → the event's single command (error if several);
//! `hook: <text>` → the unique command containing `<text>` as a substring.

use std::path::Path;

use crate::error::{Error, Result};
use crate::types::HookEvent;

/// The command strings hooks.json declares for one event.
///
/// # Errors
///
/// [`Error::Io`] / [`Error::HookResolution`] when the file is missing or malformed.
pub fn commands_for(plugin_dir: &Path, event: HookEvent) -> Result<Vec<String>> {
    let path = plugin_dir.join("hooks/hooks.json");
    let text = std::fs::read_to_string(&path).map_err(|source| Error::Io {
        operation: "read hooks.json",
        path: path.display().to_string(),
        source,
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| Error::HookResolution {
            reason: format!("{}: {e}", path.display()),
        })?;

    let mut commands = Vec::new();
    if let Some(groups) = value
        .get("hooks")
        .and_then(|h| h.get(event.as_str()))
        .and_then(serde_json::Value::as_array)
    {
        for group in groups {
            if let Some(entries) = group.get("hooks").and_then(serde_json::Value::as_array) {
                for entry in entries {
                    if let Some(command) = entry.get("command").and_then(serde_json::Value::as_str)
                    {
                        commands.push(command.to_owned());
                    }
                }
            }
        }
    }
    Ok(commands)
}

/// Resolves the one command a hook case targets.
///
/// # Errors
///
/// [`Error::HookResolution`] when zero or several commands match.
pub fn resolve(plugin_dir: &Path, event: HookEvent, reference: Option<&str>) -> Result<String> {
    let commands = commands_for(plugin_dir, event)?;
    let matched: Vec<&String> = reference.map_or_else(
        || commands.iter().collect(),
        |needle| commands.iter().filter(|c| c.contains(needle)).collect(),
    );
    match matched.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(Error::HookResolution {
            reason: format!(
                "no {} hook matches {:?} (declared: {})",
                event.as_str(),
                reference.unwrap_or("<any>"),
                commands.len()
            ),
        }),
        several => Err(Error::HookResolution {
            reason: format!(
                "{} {} hooks match {:?}; add a `hook:` substring that matches exactly one",
                several.len(),
                event.as_str(),
                reference.unwrap_or("<any>")
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::resolve;
    use crate::types::HookEvent;

    fn plugin(hooks_json: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::write(dir.path().join("hooks/hooks.json"), hooks_json).unwrap();
        dir
    }

    const TWO_HOOKS: &str = r#"{"hooks":{"PreToolUse":[
        {"matcher":"Edit|Write","hooks":[{"type":"command","command":"sh gate.sh"}]},
        {"matcher":"Read","hooks":[{"type":"command","command":"sh audit.sh"}]}
    ]}}"#;

    #[test]
    fn a_single_event_hook_resolves_without_a_reference() {
        let dir = plugin(
            r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo hi"}]}]}}"#,
        );
        assert_eq!(
            resolve(dir.path(), HookEvent::SessionStart, None).unwrap(),
            "echo hi"
        );
    }

    #[test]
    fn several_hooks_need_a_disambiguating_substring() {
        let dir = plugin(TWO_HOOKS);
        assert!(resolve(dir.path(), HookEvent::PreToolUse, None).is_err());
        assert_eq!(
            resolve(dir.path(), HookEvent::PreToolUse, Some("audit")).unwrap(),
            "sh audit.sh"
        );
    }

    #[test]
    fn zero_matches_is_an_error_naming_the_event() {
        let dir = plugin(TWO_HOOKS);
        let error = resolve(dir.path(), HookEvent::SessionEnd, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("SessionEnd"), "{error}");
    }
}
