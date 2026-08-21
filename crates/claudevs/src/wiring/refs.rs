//! The `refs` checker: every `${CLAUDE_PLUGIN_ROOT}/…` occurrence anywhere in
//! the plugin must resolve to a file that exists, and no occurrence may leave
//! the plugin root.
//!
//! The scan is textual and covers every UTF-8 file under the plugin — hooks.json
//! command strings and skill, agent and command markdown alike — because a
//! reference is a reference wherever it is written. Files that are not valid
//! UTF-8 hold no references to find and are skipped.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::error::{Error, Result};
use crate::wiring::{Finding, Severity};

/// `${CLAUDE_PLUGIN_ROOT}` followed by the path it prefixes, if any.
///
/// The tail stops at whitespace and at the quote characters that fence a
/// reference in JSON, shell and markdown; trailing sentence punctuation is
/// trimmed separately, since a path can legitimately contain `.`.
static REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\$\{CLAUDE_PLUGIN_ROOT\}(?<tail>/[^\s"'`\\]*)?"#)
        .unwrap_or_else(|_| unreachable!("the reference pattern is a valid regex"))
});

/// One `${CLAUDE_PLUGIN_ROOT}` occurrence: its 1-based line and its tail path.
pub(super) struct Occurrence {
    /// 1-based line within the file.
    pub(super) line: usize,
    /// The path after the variable, with no leading `/`; empty for a bare reference.
    pub(super) target: String,
}

/// Every occurrence in `text`, in reading order.
pub(super) fn occurrences(text: &str) -> Vec<Occurrence> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for capture in REFERENCE.captures_iter(line) {
            let tail = capture.name("tail").map_or("", |m| m.as_str());
            let target = tail
                .trim_start_matches('/')
                .trim_end_matches(['.', ',', ';', ':', ')'])
                .to_owned();
            found.push(Occurrence {
                line: index + 1,
                target,
            });
        }
    }
    found
}

/// Checks every reference under `plugin_dir`.
///
/// # Errors
///
/// [`Error::Io`] when the plugin directory cannot be walked.
pub fn check(plugin_dir: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    for entry in walkdir::WalkDir::new(plugin_dir).sort_by_file_name() {
        let entry = entry.map_err(|source| Error::Io {
            operation: "walk plugin",
            path: plugin_dir.display().to_string(),
            source: source.into(),
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let file = entry
            .path()
            .strip_prefix(plugin_dir)
            .unwrap_or_else(|_| entry.path())
            .display()
            .to_string();

        for occurrence in occurrences(&text) {
            if occurrence.target.is_empty() {
                continue;
            }
            if let Some(message) = fault(plugin_dir, &occurrence.target) {
                findings.push(Finding {
                    severity: Severity::Error,
                    checker: "refs",
                    file: file.clone(),
                    line: Some(occurrence.line),
                    message,
                });
            }
        }
    }
    Ok(findings)
}

/// What is wrong with one reference target, if anything.
fn fault(plugin_dir: &Path, target: &str) -> Option<String> {
    if target.split('/').any(|segment| segment == "..") {
        return Some(format!(
            "`${{CLAUDE_PLUGIN_ROOT}}/{target}` escapes the plugin root"
        ));
    }
    if plugin_dir.join(target).exists() {
        return None;
    }
    Some(format!("`${{CLAUDE_PLUGIN_ROOT}}/{target}` does not exist"))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::check;
    use crate::wiring::Severity;

    fn plugin(skill_body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::create_dir_all(dir.path().join("skills/demo")).unwrap();
        std::fs::write(dir.path().join("hooks/gate.sh"), "exit 0\n").unwrap();
        std::fs::write(dir.path().join("skills/demo/SKILL.md"), skill_body).unwrap();
        dir
    }

    #[test]
    fn a_reference_that_resolves_produces_no_finding() {
        let dir = plugin("run `${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh` first\n");
        assert!(check(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn a_reference_to_a_missing_file_is_an_error_naming_the_line() {
        let dir = plugin("first line\nrun ${CLAUDE_PLUGIN_ROOT}/hooks/absent.sh\n");
        let findings = check(dir.path()).unwrap();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].line, Some(2));
        assert!(
            findings[0].message.contains("hooks/absent.sh"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_traversal_is_a_finding_even_when_the_path_it_names_exists() {
        // The escape is the defect; that it happens to resolve today is not a
        // defence. Any `..` segment leaving the plugin root is a finding
        // unconditionally, because the file it reaches is not shipped with the
        // plugin and will not be there once it is installed.
        let dir = plugin("see ${CLAUDE_PLUGIN_ROOT}/../outside.txt\n");
        let outside = dir.path().parent().unwrap().join("outside.txt");
        std::fs::write(&outside, "reachable").unwrap();
        let findings = check(dir.path()).unwrap();
        let _ = std::fs::remove_file(&outside);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("escapes"), "{findings:?}");
    }

    #[test]
    fn trailing_markdown_punctuation_is_not_part_of_the_path() {
        let dir = plugin("call `${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh`.\n");
        assert!(check(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn a_sentence_final_dot_extension_still_resolves_unfenced() {
        // Unlike the backtick-fenced case above, nothing here stops the regex
        // capture before the sentence's own trailing period: the tail is
        // `scripts/okf-root.lua.` and only `trim_end_matches` recovers the
        // real filename `okf-root.lua`.
        let dir = plugin("run `${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh` first\n");
        std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
        std::fs::write(dir.path().join("scripts/okf-root.lua"), "-- lua\n").unwrap();
        std::fs::write(
            dir.path().join("skills/demo/SKILL.md"),
            "Resolve it with ${CLAUDE_PLUGIN_ROOT}/scripts/okf-root.lua.\n",
        )
        .unwrap();
        assert!(check(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn a_bare_reference_with_no_path_after_it_is_not_a_finding() {
        let dir = plugin("cd ${CLAUDE_PLUGIN_ROOT} && ls\n");
        assert!(check(dir.path()).unwrap().is_empty());
    }
}
