//! The `invocations` checker and the crate's one fenced-command parser.
//!
//! Responsibilities: [`FencedCommand`], [`parse_fenced`], [`check`].
//!
//! The grammar is the subset of fenced markdown that skill documents in this
//! repository actually use: a fence opens on a line whose first non-blank
//! characters are three or more backticks and closes on a line of backticks
//! alone; the opening line's indentation is stripped from the body, so a fence
//! nested in a numbered list parses the same as one at the margin; a body line
//! ending in `\` continues onto the next; a blank line or a `#` comment ends a
//! command. Every other body line is one command, in document order.
//!
//! Commands are kept as text, never split into argv: a referenced fenced
//! command must run *verbatim* — flags and grants included — because the
//! point of citing one is to prove the documented command is the command
//! that runs. The harness also already spawns command strings through
//! `sh -c`, so there is no argv to split into.
//!
//! [`check`]'s dead-file report exempts case files: [`crate::case::discover`]
//! finds `tests/**` entries by naming convention rather than by any other
//! file pointing at them, so they are entry points the harness runs, not
//! scripts that must be named to count as used.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::case;
use crate::error::{Error, Result};
use crate::wiring::{Finding, Severity, refs};

/// One command parsed out of a fenced block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencedCommand {
    /// 1-based line where the command starts.
    pub line: usize,
    /// The fence's info string: `sh`, `bash`, or empty.
    pub language: String,
    /// The command text, with `\` continuations joined by single spaces.
    pub command: String,
}

/// Every command in every fenced block of `markdown`, in document order.
#[must_use]
pub fn parse_fenced(markdown: &str) -> Vec<FencedCommand> {
    let mut commands = Vec::new();
    let mut open: Option<(usize, usize, String)> = None;
    let mut pending: Option<(usize, String)> = None;

    for (index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let ticks = trimmed.chars().take_while(|c| *c == '`').count();

        let Some((open_ticks, open_indent, language)) = open.clone() else {
            if ticks >= 3 {
                open = Some((ticks, indent, trimmed[ticks..].trim().to_owned()));
            }
            continue;
        };

        if ticks >= open_ticks && trimmed.trim_end_matches('`').is_empty() {
            flush(&mut commands, &mut pending, &language);
            open = None;
            continue;
        }

        let body = line
            .strip_prefix(" ".repeat(open_indent).as_str())
            .unwrap_or(trimmed)
            .trim_end();
        if body.trim().is_empty() || body.trim_start().starts_with('#') {
            flush(&mut commands, &mut pending, &language);
            continue;
        }

        let (text, continues) = body
            .strip_suffix('\\')
            .map_or((body, false), |head| (head.trim_end(), true));
        match &mut pending {
            Some((_, accumulated)) => {
                accumulated.push(' ');
                accumulated.push_str(text.trim_start());
            }
            None => pending = Some((index + 1, text.to_owned())),
        }
        if !continues {
            flush(&mut commands, &mut pending, &language);
        }
    }

    // An unclosed fence is a malformed document, not a reason to lose what it
    // held: the last command is reported rather than silently dropped.
    if let Some((_, _, language)) = open {
        flush(&mut commands, &mut pending, &language);
    }
    commands
}

/// Moves an accumulated command into the list.
fn flush(commands: &mut Vec<FencedCommand>, pending: &mut Option<(usize, String)>, language: &str) {
    if let Some((line, command)) = pending.take() {
        commands.push(FencedCommand {
            line,
            language: language.to_owned(),
            command,
        });
    }
}

/// File extensions the dead-file report covers.
const SCRIPT_EXTENSIONS: [&str; 4] = ["sh", "lua", "py", "js"];

/// Reports scripts in the plugin that nothing else in it names.
///
/// A script counts as referenced when its file name appears in any other
/// UTF-8 file of the plugin — a hooks.json command, a fenced command in a
/// skill, a sibling script. Existence of a *referenced* path is the `refs`
/// checker's job, not this one.
///
/// A file `case::discover` classifies as a case file is exempt even when
/// nothing names it: the claudevs harness finds it by glob under
/// `tests/**` and runs it directly, so "referenced by nothing" is false for
/// it — it just has no name to be referenced by.
///
/// # Errors
///
/// [`Error::Io`] when the plugin directory cannot be walked.
pub fn check(plugin_dir: &Path) -> Result<Vec<Finding>> {
    let files = readable_files(plugin_dir)?;
    let case_files = discovered_case_files(plugin_dir)?;
    let mut findings = Vec::new();

    for (path, relative, _) in &files {
        let is_script = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| SCRIPT_EXTENSIONS.contains(&e));
        if !is_script || case_files.contains(path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let referenced = files.iter().any(|(other, _, other_text)| {
            other != path && (other_text.contains(name) || mentions(other_text, name))
        });
        if !referenced {
            findings.push(Finding {
                severity: Severity::Warning,
                checker: "invocations",
                file: relative.clone(),
                line: None,
                message: format!("`{name}` is referenced by nothing in this plugin"),
            });
        }
    }
    Ok(findings)
}

/// The paths `case::discover` classifies as case files under `plugin_dir`.
///
/// A plugin with no cases at all is normal for a plugin this checker is
/// asked about, so [`Error::NoCases`] collapses to an empty set rather than
/// propagating — a caseless plugin must still get its dead-file report.
fn discovered_case_files(plugin_dir: &Path) -> Result<HashSet<PathBuf>> {
    match case::discover(plugin_dir) {
        Ok(files) => Ok(files
            .into_iter()
            .map(|file| match file {
                case::CaseFile::Yaml(path) | case::CaseFile::Lua(path) => path,
            })
            .collect()),
        Err(Error::NoCases { .. }) => Ok(HashSet::new()),
        Err(other) => Err(other),
    }
}

/// Whether any fenced command or plugin-root reference in `text` names `name`.
///
/// Plain text containment already catches most references; this adds the two
/// structured readings so a name that only ever appears inside a fence or a
/// `${CLAUDE_PLUGIN_ROOT}` tail is still seen.
fn mentions(text: &str, name: &str) -> bool {
    parse_fenced(text)
        .iter()
        .any(|command| command.command.contains(name))
        || refs::occurrences(text)
            .iter()
            .any(|occurrence| occurrence.target.ends_with(name))
}

/// Every UTF-8 file under `plugin_dir`: absolute path, plugin-relative path, text.
fn readable_files(plugin_dir: &Path) -> Result<Vec<(PathBuf, String, String)>> {
    let mut files = Vec::new();
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
        let relative = entry
            .path()
            .strip_prefix(plugin_dir)
            .unwrap_or_else(|_| entry.path())
            .display()
            .to_string();
        files.push((entry.path().to_path_buf(), relative, text));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::{check, parse_fenced};

    const SKILL: &str = "\
## Steps

1. Resolve the bundle root:

   ```sh
   airsl run --policy confined --allow-read . \\
     \"${CLAUDE_PLUGIN_ROOT}/scripts/okf-root.lua\" [explicit-path]
   ```

   Exit 2 -> relay stderr and STOP.

2. Then two more:

```bash
# a comment line is not a command
echo one
echo two
```
";

    #[test]
    fn a_continuation_joins_into_one_command() {
        let commands = parse_fenced(SKILL);
        assert_eq!(
            commands[0].command,
            "airsl run --policy confined --allow-read . \"${CLAUDE_PLUGIN_ROOT}/scripts/okf-root.lua\" [explicit-path]"
        );
        assert_eq!(commands[0].language, "sh");
        assert_eq!(commands[0].line, 6);
    }

    #[test]
    fn each_command_line_of_a_block_is_its_own_invocation_and_comments_are_not() {
        let commands = parse_fenced(SKILL);
        assert_eq!(commands.len(), 3, "{commands:?}");
        assert_eq!(commands[1].command, "echo one");
        assert_eq!(commands[2].command, "echo two");
        assert_eq!(commands[1].language, "bash");
    }

    #[test]
    fn prose_outside_a_fence_is_never_a_command() {
        let commands = parse_fenced("just prose, `echo inline` included\n");
        assert!(commands.is_empty(), "{commands:?}");
    }

    #[test]
    fn an_unclosed_fence_still_yields_what_it_opened() {
        let commands = parse_fenced("```sh\necho only\n");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, "echo only");
    }

    #[test]
    fn a_script_no_other_file_names_is_a_warning_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::write(dir.path().join("hooks/used.sh"), "exit 0\n").unwrap();
        std::fs::write(dir.path().join("hooks/orphan.sh"), "exit 0\n").unwrap();
        std::fs::write(
            dir.path().join("hooks/hooks.json"),
            r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"sh \"${CLAUDE_PLUGIN_ROOT}/hooks/used.sh\""}]}]}}"#,
        )
        .unwrap();

        let findings = check(dir.path()).unwrap();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, crate::wiring::Severity::Warning);
        assert_eq!(findings[0].file, "hooks/orphan.sh");
        assert!(findings[0].message.contains("referenced"), "{findings:?}");
    }

    #[test]
    fn a_script_naming_itself_in_its_own_body_is_still_unreferenced() {
        // Scripts routinely carry their own name — a usage line, a banner, a
        // self-referential comment. Without the `other != path` guard such a
        // file vouches for itself and can never be reported dead, so this is
        // the case that makes the guard load-bearing.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::write(
            dir.path().join("hooks/orphan.sh"),
            "# usage: sh hooks/orphan.sh\nexit 0\n",
        )
        .unwrap();

        let findings = check(dir.path()).unwrap();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, "hooks/orphan.sh");
    }

    #[test]
    fn a_script_named_only_by_a_fenced_command_counts_as_referenced() {
        // A skill that documents `sh hooks/helper.sh` with no ${CLAUDE_PLUGIN_ROOT}
        // prefix still references it; the dead-file report must not claim otherwise.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::create_dir_all(dir.path().join("skills/demo")).unwrap();
        std::fs::write(dir.path().join("hooks/helper.sh"), "exit 0\n").unwrap();
        std::fs::write(
            dir.path().join("skills/demo/SKILL.md"),
            "Run it:\n\n```sh\nsh hooks/helper.sh\n```\n",
        )
        .unwrap();
        assert!(check(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn a_discovered_case_file_is_not_reported_dead() {
        // `tests/foo_test.lua` matches the case-file naming convention that
        // `case::discover` implements — it is an entry point the harness finds
        // by glob and runs, not a script some other file must name for it to
        // count as referenced.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests")).unwrap();
        std::fs::write(dir.path().join("tests/foo_test.lua"), "return {}\n").unwrap();

        let findings = check(dir.path()).unwrap();
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_non_case_file_under_tests_named_by_nothing_is_still_reported() {
        // `tests/helper.lua` sits under tests/ but does not match the
        // case-file naming convention (`_test.lua` / `test_*.lua`), so
        // `case::discover` does not classify it as a case file. The exemption
        // must not widen to "anything under tests/" — this keeps it honest.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests")).unwrap();
        std::fs::write(dir.path().join("tests/helper.lua"), "return {}\n").unwrap();

        let findings = check(dir.path()).unwrap();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file, "tests/helper.lua");
    }
}
