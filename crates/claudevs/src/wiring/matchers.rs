//! The `matchers` checker: hooks.json declares known events and compiling
//! matcher regexes.
//!
//! The matcher is compiled with the `regex` crate, which has no lookaround and
//! no backreferences. Whether the Claude Code runtime accepts a wider flavour
//! is not established here, so a matcher using such a construct would be
//! reported although the runtime may run it; the finding says which engine
//! rejected the pattern.

use std::path::Path;
use std::str::FromStr as _;

use serde_json::Value;

use crate::types::HookEvent;
use crate::wiring::{Finding, Severity};

/// The file this checker reads, relative to the plugin root.
const HOOKS_FILE: &str = "hooks/hooks.json";

/// Checks the plugin's hooks.json, if it has one.
///
/// # Errors
///
/// Never in practice: a missing file is "nothing to check" and a malformed one
/// is a finding, because a plugin with a broken hooks.json is exactly what this
/// checker exists to report. The signature keeps the shape of its two siblings.
pub fn check(plugin_dir: &Path) -> crate::error::Result<Vec<Finding>> {
    let Ok(text) = std::fs::read_to_string(plugin_dir.join(HOOKS_FILE)) else {
        return Ok(Vec::new());
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => return Ok(vec![finding(format!("is not JSON: {error}"))]),
    };
    let Some(events) = value.get("hooks").and_then(Value::as_object) else {
        return Ok(vec![finding(String::from(
            "has no `hooks` object at the top level",
        ))]);
    };

    let mut findings = Vec::new();
    for (event, groups) in events {
        if let Err(error) = HookEvent::from_str(event) {
            findings.push(finding(error.to_string()));
        }
        for group in groups.as_array().into_iter().flatten() {
            let Some(matcher) = group.get("matcher").and_then(Value::as_str) else {
                continue;
            };
            if let Err(error) = regex::Regex::new(matcher) {
                findings.push(finding(format!(
                    "matcher `{matcher}` does not compile as a regex: {error}"
                )));
            }
        }
    }
    Ok(findings)
}

/// One finding against the hooks file.
fn finding(message: String) -> Finding {
    Finding {
        severity: Severity::Error,
        checker: "matchers",
        file: String::from(HOOKS_FILE),
        line: None,
        message,
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::check;
    use crate::wiring::Severity;

    fn plugin(hooks_json: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::write(dir.path().join("hooks/hooks.json"), hooks_json).unwrap();
        dir
    }

    #[test]
    fn a_well_formed_hooks_file_produces_no_findings() {
        let dir = plugin(
            r#"{"hooks":{"PreToolUse":[{"matcher":"Edit|Write","hooks":[{"type":"command","command":"true"}]}]}}"#,
        );
        assert!(check(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn an_unknown_event_name_is_an_error_naming_the_known_set() {
        let dir = plugin(r#"{"hooks":{"PreToolUseX":[{"hooks":[]}]}}"#);
        let findings = check(dir.path()).unwrap();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("PreToolUse"), "{findings:?}");
    }

    #[test]
    fn a_matcher_that_does_not_compile_is_an_error() {
        let dir = plugin(
            r#"{"hooks":{"PreToolUse":[{"matcher":"Edit(","hooks":[{"type":"command","command":"true"}]}]}}"#,
        );
        let findings = check(dir.path()).unwrap();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Edit("), "{findings:?}");
    }

    #[test]
    fn a_plugin_with_no_hooks_file_is_not_a_finding() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn a_hooks_file_that_is_not_json_is_one_error_not_a_crate_error() {
        let dir = plugin("{not json");
        let findings = check(dir.path()).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }
}
