//! Lua front-end: a `*_test.lua` file returns a table of named cases.
//!
//! Table values are data cases — converted to `serde_json::Value` and pushed
//! through the same [`RawCase`] path as YAML. Function values are scripted
//! cases, executed against the harness handle (`t`) in `run_scripted`.
//!
//! One engine per file: file isolation is the same property `airsl test` keeps.

use std::path::Path;

use airsl::mlua;
use airsl::{Engine, Policy, Script};

use crate::case::{Case, RawCase};
use crate::error::{Error, Result};
use crate::types::CaseName;

/// A loaded Lua case file: parsed data cases plus named scripted entries.
pub struct LuaFile {
    /// Data cases, ready to run like YAML ones.
    pub cases: Vec<Case>,
    /// Names of function-valued entries, in sorted order.
    pub scripted: Vec<CaseName>,
    /// Kept alongside `table` so the engine outlives every scripted call.
    pub(crate) engine: Engine,
    /// The table the file returned; scripted entries are looked up here.
    pub(crate) table: mlua::Table,
}

/// Loads one Lua case file on `engine` (built by the caller so the harness
/// module can be pre-installed for scripted entries).
///
/// # Errors
///
/// Returns [`Error::CaseLoad`] when the file does not load, does not return a
/// table, or a data entry fails [`Case::from_raw`].
pub fn load(engine: Engine, path: &Path) -> Result<LuaFile> {
    let fail = |reason: String| Error::CaseLoad {
        path: path.display().to_string(),
        reason,
    };

    let script = Script::from_file(path).map_err(|e| fail(e.to_string()))?;
    let table: mlua::Table = engine
        .eval_to(&script)
        .map_err(|e| fail(format!("{e}\n  a case file must return a table")))?;

    let mut cases = Vec::new();
    let mut scripted = Vec::new();
    let mut entries: Vec<(String, mlua::Value)> = table
        .pairs::<mlua::Value, mlua::Value>()
        .filter_map(std::result::Result::ok)
        .filter_map(|(k, v)| match k {
            mlua::Value::String(s) => s.to_str().ok().map(|s| (s.to_owned(), v)),
            _ => None,
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (key, value) in entries {
        let name = CaseName::new(key.as_str()).map_err(|e| fail(e.to_string()))?;
        match value {
            mlua::Value::Table(_) => {
                let json = serde_json::to_value(&value).map_err(|e| fail(e.to_string()))?;
                let raw: RawCase =
                    serde_json::from_value(json).map_err(|e| fail(format!("{key}: {e}")))?;
                cases.push(Case::from_raw(name, raw).map_err(|e| fail(format!("{key}: {e}")))?);
            }
            mlua::Value::Function(_) => scripted.push(name),
            _ => return Err(fail(format!("{key}: entries are tables or functions"))),
        }
    }

    Ok(LuaFile {
        cases,
        scripted,
        engine,
        table,
    })
}

/// The default engine for loading files that have no scripted entries (tests,
/// `migrate` round-trips): confined, no grants, stock stdlib.
///
/// # Errors
///
/// Returns [`Error::Engine`] when the engine cannot be built.
pub fn plain_engine() -> Result<Engine> {
    Engine::builder()
        .policy(Policy::confined())
        .build()
        .map_err(|e| Error::Engine {
            reason: e.to_string(),
        })
}

/// The engine for a case file: confined, zero grants, stdlib + the `t` module.
///
/// # Errors
///
/// [`Error::Engine`] when the module set or engine cannot be built.
pub(super) fn engine_for(plugin_dir: &Path, fixtures_root: &Path) -> Result<Engine> {
    let mut modules = airsl::modules::stdlib::stdlib().map_err(|e| Error::Engine {
        reason: e.to_string(),
    })?;
    modules
        .insert(Box::new(crate::harness::TModule::new(
            plugin_dir.to_path_buf(),
            fixtures_root.to_path_buf(),
        )))
        .map_err(|e| Error::Engine {
            reason: e.to_string(),
        })?;
    Engine::builder()
        .policy(Policy::confined())
        .stdlib(modules)
        .build()
        .map_err(|e| Error::Engine {
            reason: e.to_string(),
        })
}

/// Runs `names` (a subset of `loaded.scripted`) against the loaded file;
/// passing = returning.
///
/// Callers filter which names to run *before* calling this — a scripted case
/// excluded by `--case` must not execute at all, side effects included, not
/// merely go unreported.
///
/// # Errors
///
/// [`Error::Engine`] when the handle cannot be retrieved (not when a case
/// fails — that is a verdict).
pub(super) fn run_scripted(
    loaded: &LuaFile,
    names: &[CaseName],
) -> Result<Vec<(CaseName, std::result::Result<(), String>)>> {
    let handle: mlua::Table = loaded
        .engine
        .eval_to(
            &Script::from_source("return airsstack.claudevs", "claudevs-handle").map_err(|e| {
                Error::Engine {
                    reason: e.to_string(),
                }
            })?,
        )
        .map_err(|e| Error::Engine {
            reason: e.to_string(),
        })?;

    let mut results = Vec::new();
    for name in names {
        let case: mlua::Function = loaded.table.get(name.as_str()).map_err(|e| Error::Engine {
            reason: e.to_string(),
        })?;
        let outcome = case.call::<()>(handle.clone()).map_err(|e| e.to_string());
        results.push((name.clone(), outcome));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::{engine_for, load, plain_engine, run_scripted};

    fn file(body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gen_test.lua");
        std::fs::write(&path, body).unwrap();
        (dir, path)
    }

    #[test]
    fn a_data_table_parses_into_the_same_case_a_yaml_file_would() {
        let (_dir, path) = file(
            r#"return { blocks = {
                 event = "PreToolUse",
                 payload = { tool_input = { file_path = "Cargo.lock" } },
                 expect = { decision = "deny" },
               } }"#,
        );
        let loaded = load(plain_engine().unwrap(), &path).unwrap();
        assert_eq!(loaded.cases.len(), 1);
        assert_eq!(loaded.cases[0].name.as_str(), "blocks");
    }

    #[test]
    fn a_loop_generates_many_cases_from_one_file() {
        let (_dir, path) = file(
            r#"local cases = {}
               for _, f in ipairs({ "Cargo.lock", "poetry.lock" }) do
                 cases["blocks_" .. f:gsub("%.", "_")] = {
                   event = "PreToolUse",
                   payload = { tool_input = { file_path = f } },
                   expect = { decision = "deny" },
                 }
               end
               return cases"#,
        );
        assert_eq!(load(plain_engine().unwrap(), &path).unwrap().cases.len(), 2);
    }

    #[test]
    fn a_function_entry_is_collected_as_scripted_not_parsed() {
        let (_dir, path) = file("return { s = function() end }");
        let loaded = load(plain_engine().unwrap(), &path).unwrap();
        assert!(loaded.cases.is_empty());
        assert_eq!(loaded.scripted.len(), 1);
    }

    #[test]
    fn a_file_returning_no_table_is_an_author_error() {
        let (_dir, path) = file("return 42");
        assert!(load(plain_engine().unwrap(), &path).is_err());
    }

    #[test]
    fn yaml_and_lua_forms_of_one_case_are_identical() {
        // The invariant `migrate` depends on: both front-ends parse into the
        // identical `Case`, which is what makes the conversion mechanical.
        let dir = tempfile::tempdir().unwrap();
        let y = dir.path().join("same.yaml");
        std::fs::write(
            &y,
            "event: PreToolUse\npayload:\n  tool_input:\n    file_path: Cargo.lock\nexpect:\n  decision: deny\n",
        )
        .unwrap();
        let l = dir.path().join("same_test.lua");
        std::fs::write(
            &l,
            r#"return { same = {
                 event = "PreToolUse",
                 payload = { tool_input = { file_path = "Cargo.lock" } },
                 expect = { decision = "deny" },
               } }"#,
        )
        .unwrap();

        let from_yaml = crate::case::load_yaml_case(&y).unwrap();
        let from_lua = load(plain_engine().unwrap(), &l).unwrap().cases.remove(0);
        assert_eq!(from_yaml.kind, from_lua.kind);
        assert_eq!(from_yaml.expect, from_lua.expect);
    }

    #[test]
    fn a_scripted_case_runs_a_hook_through_t_and_asserts_on_it() {
        // The gate plugin driven from Lua twice to prove per-call state (each
        // t.hook call is a fresh spawn).
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();
        std::fs::write(
            dir.path().join("hooks/hooks.json"),
            r#"{"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"sh \"${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh\""}]}]}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("hooks/gate.sh"),
            "payload=$(cat)\ncase \"$payload\" in\n  *Cargo.lock*) echo 'blocked' >&2; exit 2 ;;\n  *) exit 0 ;;\nesac\n",
        )
        .unwrap();
        let path = dir.path().join("tests/scripted_test.lua");
        std::fs::write(
            &path,
            r#"return { gate_blocks_lockfiles_only = function(t)
                local blocked = t.hook("PreToolUse", { tool_input = { file_path = "Cargo.lock" } })
                assert(blocked.decision == "deny", "lockfile write must be denied")
                local clean = t.hook("PreToolUse", { tool_input = { file_path = "notes.md" } })
                assert(clean.emitted == false, "clean write must pass silently")
              end }"#,
        )
        .unwrap();

        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        let results = run_scripted(&loaded, &loaded.scripted).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1.is_ok(), "{:?}", results[0].1);
    }

    #[test]
    fn a_failing_scripted_assertion_is_a_verdict_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();
        let path = dir.path().join("tests/failing_test.lua");
        std::fs::write(
            &path,
            "return { boom = function(t) assert(false, 'nope') end }",
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        let results = run_scripted(&loaded, &loaded.scripted).unwrap();
        assert!(results[0].1.as_ref().unwrap_err().contains("nope"));
    }

    #[test]
    fn the_airsl_sandbox_refuses_fs_access_outside_the_handle() {
        // Zero grants: the airsl modules (`airsstack.fs`, ...) must refuse
        // reads even though the `t` handle works. This says nothing about
        // `t` itself, which runs host-side and is not sandboxed — see
        // `harness::t_module`'s module doc.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();
        let path = dir.path().join("tests/sandbox_test.lua");
        std::fs::write(
            &path,
            r#"return { denied = function(t)
                local ok = pcall(airsstack.fs.read, "/etc/hosts")
                assert(not ok, "the sandbox should have refused this read")
              end }"#,
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        assert!(
            run_scripted(&loaded, &loaded.scripted).unwrap()[0]
                .1
                .is_ok()
        );
    }

    #[test]
    fn t_json_refuses_a_path_outside_the_plugin_and_every_temp_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("secret.json");
        std::fs::write(&outside_file, "{}").unwrap();

        let path = dir.path().join("tests/json_denied_test.lua");
        std::fs::write(
            &path,
            format!(
                r#"return {{ refuses = function(t)
                     local ok = pcall(t.json, {:?})
                     assert(not ok, "t.json should refuse a path outside the plugin and temp projects")
                   end }}"#,
                outside_file.display().to_string()
            ),
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        assert!(
            run_scripted(&loaded, &loaded.scripted).unwrap()[0]
                .1
                .is_ok()
        );
    }

    #[test]
    fn t_json_allows_a_path_inside_the_plugin_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();
        let data_path = dir.path().join("data.json");
        std::fs::write(&data_path, r#"{"ok":true}"#).unwrap();

        let path = dir.path().join("tests/json_allowed_test.lua");
        std::fs::write(
            &path,
            format!(
                r#"return {{ reads = function(t)
                     local value = t.json({:?})
                     assert(value.ok == true, "expected ok=true")
                   end }}"#,
                data_path.display().to_string()
            ),
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        let results = run_scripted(&loaded, &loaded.scripted).unwrap();
        assert!(results[0].1.is_ok(), "{:?}", results[0].1);
    }

    #[test]
    fn t_script_refuses_a_cwd_outside_the_plugin_and_every_temp_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();
        let outside = tempfile::tempdir().unwrap();

        let path = dir.path().join("tests/cwd_denied_test.lua");
        std::fs::write(
            &path,
            format!(
                r#"return {{ refuses = function(t)
                     local ok = pcall(t.script, {{"pwd"}}, {{ cwd = {:?} }})
                     assert(not ok, "t.script should refuse a cwd outside the plugin and temp projects")
                   end }}"#,
                outside.path().display().to_string()
            ),
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        assert!(
            run_scripted(&loaded, &loaded.scripted).unwrap()[0]
                .1
                .is_ok()
        );
    }

    #[test]
    fn t_script_allows_a_cwd_inside_a_temp_project() {
        // The allow-side counterpart to the refuse test above: without this,
        // a cwd containment check that started refusing everything would go
        // red nowhere.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();

        let path = dir.path().join("tests/cwd_allowed_test.lua");
        std::fs::write(
            &path,
            r#"return { allows = function(t)
                 local project = t.project()
                 local result = t.script({ "pwd" }, { cwd = project })
                 assert(result.exit == 0, "pwd in a temp project cwd must succeed")
               end }"#,
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        let results = run_scripted(&loaded, &loaded.scripted).unwrap();
        assert!(results[0].1.is_ok(), "{:?}", results[0].1);
    }

    #[test]
    fn t_script_pipes_opts_stdin_to_the_child() {
        // `install_script` hardcoded `stdin: None` before `opts.stdin`
        // existed; without the wiring `cat` reads EOF immediately and this
        // assertion fails on an empty stdout instead of the fed payload.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();

        let path = dir.path().join("tests/script_stdin_test.lua");
        std::fs::write(
            &path,
            r#"return { feeds = function(t)
                 local result = t.script({ "cat" }, { stdin = "hello" })
                 assert(result.exit == 0, "cat must exit 0")
                 assert(
                   result.stdout == "hello",
                   "expected stdin to reach the child, got " .. tostring(result.stdout)
                 )
               end }"#,
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        let results = run_scripted(&loaded, &loaded.scripted).unwrap();
        assert!(results[0].1.is_ok(), "{:?}", results[0].1);
    }

    #[test]
    fn t_apply_fixture_allows_a_fresh_subdirectory_of_a_temp_project() {
        // Regression (canonicalizing the raw target before the containment
        // check made a subdirectory the overlay is about to create fail
        // ENOENT instead of being checked against its existing parent).
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures/edits")).unwrap();
        std::fs::write(dir.path().join("tests/fixtures/edits/new.md"), "x").unwrap();

        let path = dir.path().join("tests/apply_fixture_subdir_test.lua");
        std::fs::write(
            &path,
            r#"return { applies = function(t)
                 local project = t.project()
                 t.apply_fixture("edits", project .. "/newsub")
               end }"#,
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        let results = run_scripted(&loaded, &loaded.scripted).unwrap();
        assert!(results[0].1.is_ok(), "{:?}", results[0].1);
    }

    #[test]
    fn t_apply_fixture_refuses_the_plugin_directory_itself() {
        // A host-side write must never land in the plugin directory, even
        // though it is a legitimate *read* root for `t.json`.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures/edits")).unwrap();
        std::fs::write(dir.path().join("tests/fixtures/edits/new.md"), "x").unwrap();

        let path = dir.path().join("tests/apply_fixture_plugin_dir_test.lua");
        std::fs::write(
            &path,
            format!(
                r#"return {{ refuses = function(t)
                     local ok = pcall(t.apply_fixture, "edits", {:?})
                     assert(not ok, "t.apply_fixture must refuse to write into the plugin directory")
                   end }}"#,
                dir.path().display().to_string()
            ),
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        assert!(
            run_scripted(&loaded, &loaded.scripted).unwrap()[0]
                .1
                .is_ok()
        );
    }

    #[test]
    fn t_apply_fixture_refuses_a_dir_outside_the_plugin_and_every_temp_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures/edits")).unwrap();
        std::fs::write(dir.path().join("tests/fixtures/edits/new.md"), "x").unwrap();
        let outside = tempfile::tempdir().unwrap();

        let path = dir.path().join("tests/apply_fixture_denied_test.lua");
        std::fs::write(
            &path,
            format!(
                r#"return {{ refuses = function(t)
                     local ok = pcall(t.apply_fixture, "edits", {:?})
                     assert(not ok, "t.apply_fixture should refuse a dir outside the plugin and temp projects")
                   end }}"#,
                outside.path().display().to_string()
            ),
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        assert!(
            run_scripted(&loaded, &loaded.scripted).unwrap()[0]
                .1
                .is_ok()
        );
    }

    #[test]
    fn t_apply_fixture_allows_a_temp_project_this_module_created() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures/edits")).unwrap();
        std::fs::write(dir.path().join("tests/fixtures/edits/new.md"), "x").unwrap();

        let path = dir.path().join("tests/apply_fixture_allowed_test.lua");
        std::fs::write(
            &path,
            r#"return { applies = function(t)
                 local project = t.project()
                 t.apply_fixture("edits", project)
               end }"#,
        )
        .unwrap();
        let loaded = load(
            engine_for(dir.path(), &dir.path().join("tests/fixtures")).unwrap(),
            &path,
        )
        .unwrap();
        let results = run_scripted(&loaded, &loaded.scripted).unwrap();
        assert!(results[0].1.is_ok(), "{:?}", results[0].1);
    }
}
