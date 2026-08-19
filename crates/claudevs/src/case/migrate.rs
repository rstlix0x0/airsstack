//! `migrate`: mechanical YAML → data-Lua conversion.
//!
//! Emits the Lua table literal whose parse is byte-identical (as a [`Case`])
//! to the YAML's — the §5.2 equivalence, pinned by the round-trip test.

use std::fmt::Write as _;
use std::path::Path;

use crate::error::{Error, Result};

/// Converts one YAML case file to a `return { [<stem>] = {...} }` Lua literal.
///
/// # Errors
///
/// [`Error::CaseLoad`] when the YAML does not load as a case.
pub fn to_lua(path: &Path) -> Result<String> {
    // Validate first: migrating a file the YAML front-end rejects would launder
    // an author error into a Lua file.
    let case = crate::case::load_yaml_case(path)?;

    let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
        operation: "read",
        path: path.display().to_string(),
        source,
    })?;
    let value: serde_json::Value = serde_yaml_ng::from_str(&text).map_err(|e| Error::CaseLoad {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;

    let mut out = String::from("return {\n");
    let _ = write!(out, "  {} = ", lua_key(case.name.as_str()));
    write_value(&mut out, &value, 1);
    out.push_str(",\n}\n");

    verify_round_trip(path, &out, &case)?;

    Ok(out)
}

/// Lua 5.4's reserved words: never legal as a bare identifier, table key
/// included.
const RESERVED_WORDS: [&str; 22] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// A table key: bare identifier when legal, bracket-quoted otherwise.
///
/// A reserved word passes every other bare-identifier check (ASCII
/// alphanumeric/underscore, no leading digit) but is not a legal Lua
/// identifier — `end = 12` does not parse.
fn lua_key(key: &str) -> String {
    let bare = !key.is_empty()
        && !key.chars().next().is_some_and(|c| c.is_ascii_digit())
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !RESERVED_WORDS.contains(&key);
    if bare {
        key.to_owned()
    } else {
        format!("[{}]", lua_string(key))
    }
}

/// Reloads `lua_text` and refuses to hand it back unless it reproduces
/// `original` exactly.
///
/// The only guard `to_lua` has against emitting Lua that `--write` would
/// delete the YAML source for and then fail to reload: an empty YAML
/// sequence emits `{}`, indistinguishable on reload from an empty map, so the
/// round trip — not the emission — is what catches it.
///
/// # Errors
///
/// [`Error::CaseLoad`] naming `path` (the source YAML, not the throwaway temp
/// file this check reloads against) when the emitted Lua does not reload, or
/// reloads to something other than `original`.
fn verify_round_trip(path: &Path, lua_text: &str, original: &crate::case::Case) -> Result<()> {
    let fail = |reason: String| Error::CaseLoad {
        path: path.display().to_string(),
        reason,
    };

    let dir = tempfile::tempdir().map_err(|source| Error::Io {
        operation: "create temp dir",
        path: String::from("(round-trip check)"),
        source,
    })?;
    let lua_path = dir.path().join("round_trip_test.lua");
    std::fs::write(&lua_path, lua_text).map_err(|source| Error::Io {
        operation: "write",
        path: lua_path.display().to_string(),
        source,
    })?;

    let engine = crate::case::plain_engine()?;
    let mut loaded = crate::case::load_lua_file(engine, &lua_path).map_err(|e| {
        fail(format!(
            "the migrated Lua does not reload cleanly: {}",
            strip_case_load_path(&e)
        ))
    })?;

    if loaded.cases.len() != 1 {
        return Err(fail(format!(
            "the migrated Lua reloaded {} cases, expected exactly 1",
            loaded.cases.len()
        )));
    }
    let reloaded = loaded.cases.remove(0);
    if reloaded != *original {
        return Err(fail(format!(
            "the migrated Lua reloads but does not reproduce the original case ({} differed)",
            differing_fields(&reloaded, original).join(", ")
        )));
    }
    Ok(())
}

/// The reason a nested [`Error::CaseLoad`] reports, without its own `path`
/// prefix — the round-trip check reloads against a throwaway temp file, and
/// naming that internal path a second time (nested under the outer
/// [`Error::CaseLoad`] this feeds into, which already names the source YAML)
/// would only be noise. Falls back to the full message for every other
/// error variant.
fn strip_case_load_path(error: &Error) -> String {
    let full = error.to_string();
    match full.split_once("`: ") {
        Some((_, reason)) => reason.to_owned(),
        None => full,
    }
}

/// Which of a case's compared fields differ, for a diagnostic message.
fn differing_fields(
    reloaded: &crate::case::Case,
    original: &crate::case::Case,
) -> Vec<&'static str> {
    let mut differed = Vec::new();
    if reloaded.name != original.name {
        differed.push("name");
    }
    if reloaded.kind != original.kind {
        differed.push("kind");
    }
    if reloaded.project != original.project {
        differed.push("project");
    }
    if reloaded.expect != original.expect {
        differed.push("expect");
    }
    differed
}

/// A double-quoted Lua string literal.
fn lua_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Writes `value` as a Lua literal at `depth` (two-space indent).
fn write_value(out: &mut String, value: &serde_json::Value, depth: usize) {
    let pad = "  ".repeat(depth + 1);
    let close = "  ".repeat(depth);
    match value {
        serde_json::Value::Null => out.push_str("nil"),
        serde_json::Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        serde_json::Value::Number(n) => out.push_str(&n.to_string()),
        serde_json::Value::String(s) => out.push_str(&lua_string(s)),
        serde_json::Value::Array(items) => {
            out.push_str("{\n");
            for item in items {
                out.push_str(&pad);
                write_value(out, item, depth + 1);
                out.push_str(",\n");
            }
            out.push_str(&close);
            out.push('}');
        }
        serde_json::Value::Object(map) => {
            out.push_str("{\n");
            for (key, item) in map {
                let _ = write!(out, "{pad}{} = ", lua_key(key));
                write_value(out, item, depth + 1);
                out.push_str(",\n");
            }
            out.push_str(&close);
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]
    #![expect(
        clippy::panic,
        reason = "an unexpected Ok in a match-on-Err setup panics by design"
    )]

    use super::to_lua;

    #[test]
    fn strip_case_load_path_drops_a_nested_case_loads_own_wrapping_prefix() {
        // The round-trip check reloads against a throwaway temp file wrapped
        // in its own `Error::CaseLoad { path: <temp file>, .. }`; nesting
        // that verbatim under `verify_round_trip`'s own `Error::CaseLoad {
        // path: <source yaml>, .. }` would name the temp file a second,
        // redundant time instead of just the source YAML already named by
        // the outer error.
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.lua");
        std::fs::write(&bad, "return 42").unwrap();
        let Err(err) = crate::case::load_lua_file(crate::case::plain_engine().unwrap(), &bad)
        else {
            panic!("a file returning `42` must fail to load");
        };

        let stripped = super::strip_case_load_path(&err);
        assert!(
            !stripped.starts_with("cannot load case"),
            "the nested `Error::CaseLoad`'s own wrapping prefix must be gone: {stripped}"
        );
        assert!(
            stripped.contains("a case file must return a table"),
            "{stripped}"
        );
    }

    #[test]
    fn differing_fields_names_which_parts_of_a_case_mismatched() {
        use crate::case::{Case, Expectations, FixtureRef, Invocation};
        use crate::types::CaseName;

        let base = Case {
            name: CaseName::new("t").unwrap(),
            kind: crate::case::CaseKind::Script {
                invocation: Invocation {
                    argv: vec![String::from("true")],
                    env: std::collections::BTreeMap::default(),
                },
            },
            project: None,
            expect: Expectations::default(),
        };

        let same = base.clone();
        assert!(super::differing_fields(&base, &same).is_empty());

        let mut different_expect = base.clone();
        different_expect.expect = Expectations {
            exit: Some(0),
            ..Expectations::default()
        };
        assert_eq!(
            super::differing_fields(&base, &different_expect),
            vec!["expect"]
        );

        let mut different_kind = base.clone();
        different_kind.kind = crate::case::CaseKind::Script {
            invocation: Invocation {
                argv: vec![String::from("false")],
                env: std::collections::BTreeMap::default(),
            },
        };
        assert_eq!(
            super::differing_fields(&base, &different_kind),
            vec!["kind"]
        );

        let mut different_project = base.clone();
        different_project.project = Some(FixtureRef(String::from("repo")));
        assert_eq!(
            super::differing_fields(&base, &different_project),
            vec!["project"]
        );
    }

    #[test]
    fn the_emitted_lua_parses_back_to_the_identical_case() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("blocks-lockfile.yaml");
        std::fs::write(
            &yaml,
            "event: PreToolUse\npayload:\n  tool_input:\n    file_path: Cargo.lock\nexpect:\n  decision: deny\n  stderr_contains: lockfile\n",
        )
        .unwrap();
        let original = crate::case::load_yaml_case(&yaml).unwrap();

        let lua_text = to_lua(&yaml).unwrap();
        let lua_path = dir.path().join("blocks_test.lua");
        std::fs::write(&lua_path, &lua_text).unwrap();
        let loaded =
            crate::case::load_lua_file(crate::case::plain_engine().unwrap(), &lua_path).unwrap();

        assert_eq!(loaded.cases.len(), 1, "{lua_text}");
        assert_eq!(loaded.cases[0].kind, original.kind);
        assert_eq!(loaded.cases[0].expect, original.expect);
    }

    #[test]
    fn a_yaml_the_front_end_rejects_is_not_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("bad.yaml");
        std::fs::write(&yaml, "event: PreToolUse\nexpct: {}\n").unwrap();
        assert!(to_lua(&yaml).is_err());
    }

    #[test]
    fn a_lua_reserved_word_key_is_bracket_quoted_not_bare() {
        assert_eq!(super::lua_key("end"), "[\"end\"]");
        assert_eq!(super::lua_key("nil"), "[\"nil\"]");
        assert_eq!(super::lua_key("normal_key"), "normal_key");
    }

    #[test]
    fn a_reserved_word_key_migrates_to_lua_that_reloads_cleanly() {
        // `end`/`nil` are Lua reserved words; a bare `end = 12` does not parse.
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("reserved.yaml");
        std::fs::write(
            &yaml,
            "event: PreToolUse\npayload:\n  tool_input:\n    edits:\n      end: 12\n      nil: 3\nexpect:\n  decision: allow\n",
        )
        .unwrap();
        let original = crate::case::load_yaml_case(&yaml).unwrap();

        let lua_text = to_lua(&yaml).unwrap();
        assert!(lua_text.contains("[\"end\"] = 12"), "{lua_text}");
        assert!(lua_text.contains("[\"nil\"] = 3"), "{lua_text}");

        let lua_path = dir.path().join("reserved_test.lua");
        std::fs::write(&lua_path, &lua_text).unwrap();
        let loaded =
            crate::case::load_lua_file(crate::case::plain_engine().unwrap(), &lua_path).unwrap();
        assert_eq!(loaded.cases.len(), 1, "{lua_text}");
        assert_eq!(loaded.cases[0].kind, original.kind);
    }

    #[test]
    fn an_empty_sequence_refuses_to_migrate_instead_of_emitting_an_ambiguous_table() {
        // `{}` cannot round-trip as an empty *sequence*; the round-trip guard
        // must reject it before `--write` deletes the YAML source.
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("nulls.yaml");
        std::fs::write(&yaml, "event: PreToolUse\nexpect:\n  files_exist: []\n").unwrap();
        let error = to_lua(&yaml).unwrap_err().to_string();
        assert!(error.contains("nulls"), "{error}");
    }
}
