//! The `claudevs` airsl host module — the harness handle scripted cases receive.
//!
//! Installed as `airsstack.claudevs` on the per-file engine and passed to each
//! scripted case as its `t` argument. Functions:
//!
//! - `t.fixture(name)` → path of a fresh temp project seeded from the fixture
//! - `t.project()` → path of a fresh empty temp project
//! - `t.apply_fixture(name, dir)` → overlay a fixture into a directory
//! - `t.hook(ref, payload?)` → run a hook: `{exit, stdout, stderr, decision?, context?, emitted}`
//!   (`ref` is an event name, or a command substring unique across all events)
//! - `t.script(argv, opts?)` → run argv (`opts.env`, `opts.cwd`): `{exit, stdout, stderr}`
//! - `t.json(path)` → decode a JSON file
//!
//! Only the Lua *itself* is sandboxed: the airsl policy is `confined` with
//! zero grants, so a scripted case cannot reach `airsstack.fs`,
//! `airsstack.process`, or any other airsl module directly. The handle above
//! is not part of that sandbox — every function here runs host-side, as
//! ordinary Rust: `t.script` spawns any argv with the ambient environment,
//! and the child it starts is not confined in any way. A `*_test.lua` file
//! is therefore trusted code, on the same footing as any shell script the
//! author runs locally — not an untrusted payload the harness defends
//! against.
//!
//! `t.json`'s path (a read) and `t.script`'s `opts.cwd` (a spawn's working
//! directory, unsandboxed regardless) are checked against the plugin
//! directory *or* the temp projects this module has created
//! ([`ensure_contained`]) before they are touched. `t.apply_fixture`'s `dir`
//! is a write, and is checked more narrowly ([`ensure_write_contained`]):
//! only a temp project this module created, never the plugin directory —
//! the plugin's own files are never a legitimate overlay target. Every one
//! of these checks guards against an accident — a fixture argument
//! resolving to the wrong directory, a stray absolute path — not against a
//! hostile case file, which reaches `t.script` regardless and can run
//! whatever it likes.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use airsl::mlua;
use airsl::mlua::LuaSerdeExt as _;
use airsl::{HostModule, InstallContext};

use crate::case::Decision;
use crate::error::Error;
use crate::harness::{
    DEFAULT_TIMEOUT, Project, base_env, default_payload, merge, observe, overlay_into, run,
    run_shell, substitute_project,
};
use crate::types::HookEvent;

/// The host module carrying the harness handle.
pub struct TModule {
    name: airsl::ModuleName,
    plugin_dir: PathBuf,
    fixtures_root: PathBuf,
    /// Keeps every temp project alive until the engine drops.
    projects: Arc<Mutex<Vec<Project>>>,
}

impl TModule {
    /// Builds the module for one plugin.
    ///
    /// # Panics
    ///
    /// Never in practice: the module name is a valid literal.
    #[must_use]
    pub fn new(plugin_dir: PathBuf, fixtures_root: PathBuf) -> Self {
        Self {
            name: airsl::ModuleName::new("claudevs")
                .unwrap_or_else(|_| unreachable!("`claudevs` is a valid module name")),
            plugin_dir,
            fixtures_root,
            projects: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// Converts a crate error into a Lua error for raising inside a case.
fn lua_err(error: Error) -> mlua::Error {
    mlua::Error::external(error)
}

/// Converts a Lua-side installation failure into the `claudevs` module's
/// [`airsl::Error::ModuleInstall`].
fn install_err(error: &mlua::Error) -> airsl::Error {
    airsl::Error::ModuleInstall {
        module: String::from("claudevs"),
        reason: error.to_string(),
    }
}

/// Installs `t.fixture(name)` and `t.project()`.
fn install_project_functions(
    lua: &mlua::Lua,
    table: &mlua::Table,
    fixtures_root: &std::path::Path,
    projects: &Arc<Mutex<Vec<Project>>>,
) -> airsl::Result<()> {
    let (fixtures, projects_for_fixture) = (fixtures_root.to_path_buf(), Arc::clone(projects));
    let fixture = lua
        .create_function(move |_, name: String| {
            let project = Project::from_fixture(&fixtures, &name).map_err(lua_err)?;
            let path = project.path().display().to_string();
            projects_for_fixture
                .lock()
                .map_err(|_| mlua::Error::external("poisoned"))?
                .push(project);
            Ok(path)
        })
        .map_err(|e| install_err(&e))?;
    table.set("fixture", fixture).map_err(|e| install_err(&e))?;

    let projects_for_empty = Arc::clone(projects);
    let project_fn = lua
        .create_function(move |_, ()| {
            let project = Project::empty().map_err(lua_err)?;
            let path = project.path().display().to_string();
            projects_for_empty
                .lock()
                .map_err(|_| mlua::Error::external("poisoned"))?
                .push(project);
            Ok(path)
        })
        .map_err(|e| install_err(&e))?;
    table
        .set("project", project_fn)
        .map_err(|e| install_err(&e))
}

/// Installs `t.apply_fixture(name, dir)`.
fn install_apply_fixture(
    lua: &mlua::Lua,
    table: &mlua::Table,
    fixtures_root: &std::path::Path,
    projects: &Arc<Mutex<Vec<Project>>>,
) -> airsl::Result<()> {
    let fixtures = fixtures_root.to_path_buf();
    let projects = Arc::clone(projects);
    let apply = lua
        .create_function(move |_, (name, dir): (String, String)| {
            // A write, unlike `t.json`'s read: only a temp project this
            // module created may be the target, never the plugin directory.
            let checked = ensure_write_contained(std::path::Path::new(&dir), &projects)?;
            overlay_into(&fixtures, &name, &checked).map_err(lua_err)
        })
        .map_err(|e| install_err(&e))?;
    table
        .set("apply_fixture", apply)
        .map_err(|e| install_err(&e))
}

/// Installs `t.hook(ref, payload?)`.
fn install_hook(
    lua: &mlua::Lua,
    table: &mlua::Table,
    plugin_dir: &std::path::Path,
) -> airsl::Result<()> {
    let plugin_dir = plugin_dir.to_path_buf();
    let hook = lua
        .create_function(
            move |lua, (reference, payload): (String, Option<mlua::Table>)| {
                let (event, command) = resolve_ref(&plugin_dir, &reference).map_err(lua_err)?;

                let project = Project::empty().map_err(lua_err)?;
                let project_str = project.path().display().to_string();
                let mut value = default_payload(event);
                if let Some(table) = payload {
                    let overlay = serde_json::to_value(mlua::Value::Table(table))
                        .map_err(mlua::Error::external)?;
                    merge(&mut value, &overlay);
                }
                substitute_project(&mut value, &project_str);

                let env = base_env(&plugin_dir, project.path());
                let captured = run_shell(
                    &command,
                    project.path(),
                    &env,
                    Some(&value.to_string()),
                    DEFAULT_TIMEOUT,
                )
                .map_err(lua_err)?;
                let observed = observe(event, &captured);

                let result = lua.create_table()?;
                result.set("exit", observed.exit)?;
                result.set("stdout", observed.stdout)?;
                result.set("stderr", observed.stderr)?;
                result.set("emitted", observed.emitted)?;
                if let Some(decision) = observed.decision {
                    result.set("decision", decision_name(decision))?;
                }
                if let Some(context) = observed.context {
                    result.set("context", context)?;
                }
                Ok(result)
            },
        )
        .map_err(|e| install_err(&e))?;
    table.set("hook", hook).map_err(|e| install_err(&e))
}

/// Installs `t.script(argv, opts?)`.
fn install_script(
    lua: &mlua::Lua,
    table: &mlua::Table,
    plugin_dir: &std::path::Path,
    projects: &Arc<Mutex<Vec<Project>>>,
) -> airsl::Result<()> {
    let (plugin_dir, projects) = (plugin_dir.to_path_buf(), Arc::clone(projects));
    let script = lua
        .create_function(
            move |lua, (argv, opts): (Vec<String>, Option<mlua::Table>)| {
                let cwd: String = opts
                    .as_ref()
                    .and_then(|o| o.get::<Option<String>>("cwd").ok().flatten())
                    .unwrap_or_else(|| plugin_dir.display().to_string());
                let checked_cwd =
                    ensure_contained(std::path::Path::new(&cwd), &plugin_dir, &projects)?;
                let mut env = base_env(&plugin_dir, &checked_cwd);
                if let Some(extra) =
                    opts.and_then(|o| o.get::<Option<mlua::Table>>("env").ok().flatten())
                {
                    for pair in extra.pairs::<String, String>() {
                        let (key, value) = pair?;
                        env.insert(key, value);
                    }
                }
                let captured =
                    run(&argv, &checked_cwd, &env, None, DEFAULT_TIMEOUT).map_err(lua_err)?;
                let result = lua.create_table()?;
                result.set("exit", captured.exit)?;
                result.set("stdout", captured.stdout)?;
                result.set("stderr", captured.stderr)?;
                Ok(result)
            },
        )
        .map_err(|e| install_err(&e))?;
    table.set("script", script).map_err(|e| install_err(&e))
}

/// Installs `t.json(path)`.
fn install_json(
    lua: &mlua::Lua,
    table: &mlua::Table,
    plugin_dir: &std::path::Path,
    projects: &Arc<Mutex<Vec<Project>>>,
) -> airsl::Result<()> {
    let (plugin_dir, projects) = (plugin_dir.to_path_buf(), Arc::clone(projects));
    let json = lua
        .create_function(move |lua, path: String| {
            let checked = ensure_contained(std::path::Path::new(&path), &plugin_dir, &projects)?;
            let text = std::fs::read_to_string(&checked)
                .map_err(|e| mlua::Error::external(format!("{path}: {e}")))?;
            let value: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| mlua::Error::external(format!("{path}: {e}")))?;
            lua.to_value(&value)
        })
        .map_err(|e| install_err(&e))?;
    table.set("json", json).map_err(|e| install_err(&e))
}

impl HostModule for TModule {
    fn name(&self) -> &airsl::ModuleName {
        &self.name
    }

    fn install(
        &self,
        lua: &mlua::Lua,
        table: &mlua::Table,
        _context: &InstallContext<'_>,
    ) -> airsl::Result<()> {
        install_project_functions(lua, table, &self.fixtures_root, &self.projects)?;
        install_apply_fixture(lua, table, &self.fixtures_root, &self.projects)?;
        install_hook(lua, table, &self.plugin_dir)?;
        install_script(lua, table, &self.plugin_dir, &self.projects)?;
        install_json(lua, table, &self.plugin_dir, &self.projects)
    }
}

/// `ref` is an event name, or a substring unique across all events' commands.
fn resolve_ref(
    plugin_dir: &std::path::Path,
    reference: &str,
) -> crate::error::Result<(HookEvent, String)> {
    if let Ok(event) = reference.parse::<HookEvent>() {
        let command = crate::harness::resolve_hook(plugin_dir, event, None)?;
        return Ok((event, command));
    }
    let mut matches = Vec::new();
    for event in [
        HookEvent::PreToolUse,
        HookEvent::PostToolUse,
        HookEvent::UserPromptSubmit,
        HookEvent::SessionStart,
        HookEvent::SessionEnd,
    ] {
        if let Ok(command) = crate::harness::resolve_hook(plugin_dir, event, Some(reference)) {
            matches.push((event, command));
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(Error::HookResolution {
            reason: format!("`{reference}` matches no hook command"),
        }),
        n => Err(Error::HookResolution {
            reason: format!("`{reference}` matches {n} hook commands across events"),
        }),
    }
}

/// The lowercase wire name of a decision.
const fn decision_name(decision: Decision) -> &'static str {
    match decision {
        Decision::Allow => "allow",
        Decision::Deny => "deny",
        Decision::Ask => "ask",
        Decision::Defer => "defer",
    }
}

/// Walks `path` up to its nearest existing ancestor, returning that ancestor
/// canonicalized together with the non-existent tail below it (empty when
/// `path` itself already exists).
///
/// The containment check has to land on a path an operation (`copy_tree`,
/// e.g.) is about to *create* — canonicalizing `path` itself would fail
/// ENOENT for exactly the case that is supposed to work: a fresh subdirectory
/// under an already-contained root.
fn nearest_existing_ancestor(path: &std::path::Path) -> std::io::Result<(PathBuf, PathBuf)> {
    let mut tail = PathBuf::new();
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return Ok((std::fs::canonicalize(&current)?, tail));
        }
        match (current.parent(), current.file_name()) {
            (Some(parent), Some(name)) if parent != current => {
                tail = PathBuf::from(name).join(&tail);
                current = parent.to_path_buf();
            }
            // No parent left to try (or it does not shrink further): let
            // canonicalize on the original path surface the real io error.
            _ => return std::fs::canonicalize(&current).map(|c| (c, PathBuf::new())),
        }
    }
}

/// Rejoins an ancestor and its tail, without appending a spurious trailing
/// separator when `tail` is empty — `path.join(PathBuf::new())` still adds
/// one, which turns a plain file into an `ENOTDIR` when it is later opened.
fn rejoin(ancestor: PathBuf, tail: PathBuf) -> PathBuf {
    if tail.as_os_str().is_empty() {
        ancestor
    } else {
        ancestor.join(tail)
    }
}

/// Whether `ancestor` (already canonicalized) sits inside a temp project this
/// module created.
fn within_a_temp_project(
    ancestor: &std::path::Path,
    projects: &Arc<Mutex<Vec<Project>>>,
) -> std::result::Result<bool, mlua::Error> {
    let guard = projects
        .lock()
        .map_err(|_| mlua::Error::external("poisoned"))?;
    Ok(guard.iter().any(|project| {
        std::fs::canonicalize(project.path()).is_ok_and(|root| ancestor.starts_with(&root))
    }))
}

/// Refuses a host-side path outside the plugin directory or a temp project
/// this module created. Used for reads (`t.json`) and `t.script`'s cwd, where
/// landing in the plugin directory itself is legitimate.
///
/// This guards against an accident (a typo, a fixture resolving to the wrong
/// directory), not malice — see the module doc: a scripted case file is
/// trusted code and `t.script` can already run anything.
///
/// # Errors
///
/// An [`mlua::Error`] naming `path` and the allowed roots (the containment
/// rule) when neither root contains `path`'s nearest existing ancestor, or
/// when `plugin_dir` does not resolve to an existing filesystem entry.
fn ensure_contained(
    path: &std::path::Path,
    plugin_dir: &std::path::Path,
    projects: &Arc<Mutex<Vec<Project>>>,
) -> std::result::Result<PathBuf, mlua::Error> {
    let (ancestor, tail) = nearest_existing_ancestor(path)
        .map_err(|e| mlua::Error::external(format!("`{}`: {e}", path.display())))?;
    let plugin_canonical = std::fs::canonicalize(plugin_dir)
        .map_err(|e| mlua::Error::external(format!("`{}`: {e}", plugin_dir.display())))?;

    if ancestor.starts_with(&plugin_canonical) || within_a_temp_project(&ancestor, projects)? {
        return Ok(rejoin(ancestor, tail));
    }

    Err(mlua::Error::external(format!(
        "`{}` is outside the plugin directory (`{}`) and every temp project this module created",
        path.display(),
        plugin_dir.display()
    )))
}

/// Refuses a host-side *write* target outside every temp project this module
/// created — stricter than [`ensure_contained`]: a write must never land in
/// the plugin directory. `t.apply_fixture`'s `dir` argument is the module's
/// one host-side write path (see the module doc).
///
/// # Errors
///
/// An [`mlua::Error`] naming `path` and the allowed roots (the containment
/// rule: every temp project this module created, never the plugin
/// directory) when `path`'s nearest existing ancestor sits outside all of
/// them.
fn ensure_write_contained(
    path: &std::path::Path,
    projects: &Arc<Mutex<Vec<Project>>>,
) -> std::result::Result<PathBuf, mlua::Error> {
    let (ancestor, tail) = nearest_existing_ancestor(path)
        .map_err(|e| mlua::Error::external(format!("`{}`: {e}", path.display())))?;

    if within_a_temp_project(&ancestor, projects)? {
        return Ok(rejoin(ancestor, tail));
    }

    Err(mlua::Error::external(format!(
        "`{}` is outside every temp project this module created (host-side writes may not target the plugin directory)",
        path.display()
    )))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use std::sync::{Arc, Mutex};

    use super::{decision_name, ensure_contained, resolve_ref};
    use crate::case::Decision;
    use crate::harness::Project;
    use crate::types::HookEvent;

    fn plugin(hooks_json: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::write(dir.path().join("hooks/hooks.json"), hooks_json).unwrap();
        dir
    }

    const TWO_EVENTS: &str = r#"{"hooks":{
        "PreToolUse":[{"hooks":[{"type":"command","command":"sh gate.sh"}]}],
        "PostToolUse":[{"hooks":[{"type":"command","command":"sh audit.sh"}]}]
    }}"#;

    #[test]
    fn resolve_ref_matches_an_event_name_directly() {
        let dir = plugin(TWO_EVENTS);
        let (event, command) = resolve_ref(dir.path(), "PreToolUse").unwrap();
        assert_eq!(event, HookEvent::PreToolUse);
        assert_eq!(command, "sh gate.sh");
    }

    #[test]
    fn resolve_ref_matches_a_command_substring_unique_across_events() {
        let dir = plugin(TWO_EVENTS);
        let (event, command) = resolve_ref(dir.path(), "audit").unwrap();
        assert_eq!(event, HookEvent::PostToolUse);
        assert_eq!(command, "sh audit.sh");
    }

    #[test]
    fn resolve_ref_reports_a_zero_match_naming_the_reference() {
        let dir = plugin(TWO_EVENTS);
        let error = resolve_ref(dir.path(), "nonexistent")
            .unwrap_err()
            .to_string();
        assert!(error.contains("nonexistent"), "{error}");
    }

    #[test]
    fn resolve_ref_reports_a_reference_matching_across_events() {
        let dir = plugin(
            r#"{"hooks":{
                "PreToolUse":[{"hooks":[{"type":"command","command":"sh both.sh"}]}],
                "PostToolUse":[{"hooks":[{"type":"command","command":"sh both.sh"}]}]
            }}"#,
        );
        let error = resolve_ref(dir.path(), "both").unwrap_err().to_string();
        assert!(error.contains('2'), "{error}");
    }

    #[test]
    fn decision_name_is_the_lowercase_wire_name() {
        assert_eq!(decision_name(Decision::Allow), "allow");
        assert_eq!(decision_name(Decision::Deny), "deny");
        assert_eq!(decision_name(Decision::Ask), "ask");
        assert_eq!(decision_name(Decision::Defer), "defer");
    }

    #[test]
    fn a_path_under_the_plugin_directory_is_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("claudevs.toml");
        std::fs::write(&file, "").unwrap();
        let projects = Arc::new(Mutex::new(Vec::new()));
        assert!(ensure_contained(&file, dir.path(), &projects).is_ok());
    }

    #[test]
    fn a_path_under_a_temp_project_this_module_created_is_allowed() {
        let plugin_dir = tempfile::tempdir().unwrap();
        let projects = Arc::new(Mutex::new(Vec::new()));
        let project = Project::empty().unwrap();
        let file = project.path().join("data.json");
        std::fs::write(&file, "{}").unwrap();
        projects.lock().unwrap().push(project);
        assert!(ensure_contained(&file, plugin_dir.path(), &projects).is_ok());
    }

    #[test]
    fn a_not_yet_existing_subdir_of_a_temp_project_is_allowed() {
        // Regression: canonicalizing the raw path before the containment
        // check made a fresh subdir (the target `copy_tree` is about to
        // create) fail ENOENT instead of being checked against its nearest
        // existing ancestor.
        let plugin_dir = tempfile::tempdir().unwrap();
        let projects = Arc::new(Mutex::new(Vec::new()));
        let project = Project::empty().unwrap();
        let project_root = project.path().to_path_buf();
        let target = project_root.join("newsub");
        projects.lock().unwrap().push(project);

        let expected = std::fs::canonicalize(&project_root).unwrap().join("newsub");
        let checked = ensure_contained(&target, plugin_dir.path(), &projects).unwrap();
        assert_eq!(checked, expected);
        assert!(
            !target.exists(),
            "the check itself must not create anything"
        );
    }

    #[test]
    fn a_path_outside_the_plugin_and_every_temp_project_is_refused() {
        let plugin_dir = tempfile::tempdir().unwrap();
        let projects = Arc::new(Mutex::new(Vec::new()));
        let error = ensure_contained(
            std::path::Path::new("/etc/hosts"),
            plugin_dir.path(),
            &projects,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("/etc/hosts"), "{error}");
        assert!(
            error.contains(&plugin_dir.path().display().to_string()),
            "{error}"
        );
    }

    #[test]
    fn a_not_yet_existing_path_outside_every_root_is_still_refused() {
        let plugin_dir = tempfile::tempdir().unwrap();
        let projects = Arc::new(Mutex::new(Vec::new()));
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("nope/deeper");

        let error = ensure_contained(&target, plugin_dir.path(), &projects)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&plugin_dir.path().display().to_string()),
            "{error}"
        );
    }
}
