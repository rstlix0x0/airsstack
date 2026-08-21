//! Running a plugin's suite: discovery → per-case execution → report data.
//!
//! Responsibilities: [`run_suite`], [`run_case`], [`SuiteOptions`],
//! [`SuiteReport`], [`CaseOutcome`].

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::path::Path;

use crate::case::{Case, CaseFile, CaseKind, Expectations, Invocation, discover};
use crate::error::{Error, Result};
use crate::harness::{
    DEFAULT_TIMEOUT, Observed, Project, Verdict, base_env, default_payload, judge, merge, observe,
    overlay_into, resolve_hook, run, run_shell, substitute_project,
};
use crate::native::{NativeOutcome, run_declared};
use crate::types::HookEvent;

/// Knobs for one suite run.
#[derive(Debug, Clone, Default)]
pub struct SuiteOptions {
    /// Only run cases whose name contains this substring.
    pub case_filter: Option<String>,
}

/// One case's reported outcome.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CaseOutcome {
    /// The case name.
    pub name: String,
    /// Pass or the mismatch list.
    pub verdict: Verdict,
}

/// Everything one run produced.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SuiteReport {
    /// Case outcomes, in discovery order.
    pub outcomes: Vec<CaseOutcome>,
    /// Declared native suites' outcomes.
    pub native: Vec<NativeOutcome>,
}

impl SuiteReport {
    /// Whether everything passed.
    #[must_use]
    pub fn all_green(&self) -> bool {
        self.outcomes
            .iter()
            .all(|o| matches!(o.verdict, Verdict::Pass))
            && self.native.iter().all(|n| n.exit == 0)
    }
}

/// Runs the whole suite of the plugin at `plugin_dir`.
///
/// # Errors
///
/// Errors are the *inability to run* (discovery failure, unloadable case
/// files, unresolvable hooks); failing cases are outcomes, not errors.
pub fn run_suite(plugin_dir: &Path, options: &SuiteOptions) -> Result<SuiteReport> {
    // Children run inside the temp project, so a relative plugin path (the
    // CLI's default `.`) would make `CLAUDE_PLUGIN_ROOT` resolve against the
    // wrong directory — absolutize once here for every downstream consumer.
    let plugin_dir = plugin_dir.canonicalize().map_err(|source| Error::Io {
        operation: "resolve plugin dir",
        path: plugin_dir.display().to_string(),
        source,
    })?;
    let plugin_dir = plugin_dir.as_path();
    let fixtures_root = plugin_dir.join("tests/fixtures");
    let mut outcomes = Vec::new();

    for file in discover(plugin_dir)? {
        match file {
            CaseFile::Yaml(path) => {
                let case = crate::case::load_yaml_case(&path)?;
                if selected(options, case.name.as_str()) {
                    outcomes.push(run_case(plugin_dir, &fixtures_root, &case)?);
                }
            }
            CaseFile::Lua(path) => {
                outcomes.extend(crate::case::run_lua_file(
                    plugin_dir,
                    &fixtures_root,
                    &path,
                    options,
                )?);
            }
        }
    }

    let native = run_declared(plugin_dir)?;
    Ok(SuiteReport { outcomes, native })
}

/// Runs the whole suite against a throwaway copy of the plugin in the shape it
/// has once installed (`--installed`).
///
/// The cases are the same ones [`run_suite`] runs; only the context differs —
/// `CLAUDE_PLUGIN_ROOT` points at the cache copy, so a path that resolves only
/// in the source checkout comes apart here.
///
/// # Errors
///
/// [`Error::Manifest`] or [`Error::Layout`] when the copy cannot be built, and
/// then the same conditions as [`run_suite`].
pub fn run_suite_installed(plugin_dir: &Path, options: &SuiteOptions) -> Result<SuiteReport> {
    // The layout owns a temp dir; holding it until the run finishes is what
    // keeps the copy on disk for the children the harness spawns.
    let installed = crate::layout::Installed::materialize(plugin_dir)?;
    run_suite(installed.plugin_root(), options)
}

/// Whether `name` passes the filter.
pub(crate) fn selected(options: &SuiteOptions, name: &str) -> bool {
    options
        .case_filter
        .as_deref()
        .is_none_or(|needle| name.contains(needle))
}

/// Runs one data case.
///
/// # Errors
///
/// Same conditions as [`run_suite`].
pub fn run_case(plugin_dir: &Path, fixtures_root: &Path, case: &Case) -> Result<CaseOutcome> {
    let project = match &case.project {
        Some(fixture) => Project::from_fixture(fixtures_root, &fixture.0)?,
        None => Project::empty()?,
    };
    let project_str = project.path().display().to_string();
    let env = base_env(plugin_dir, project.path());

    let verdict = match &case.kind {
        CaseKind::Hook {
            event,
            hook,
            payload,
            payload_raw,
        } => {
            let command = resolve_hook(plugin_dir, *event, hook.as_deref())?;
            let stdin = stdin_for(
                *event,
                payload.as_ref(),
                payload_raw.as_deref(),
                &project_str,
            );
            let captured = run_shell(
                &command,
                project.path(),
                &env,
                Some(&stdin),
                DEFAULT_TIMEOUT,
            )?;
            judge(&case.expect, &observe(*event, &captured), project.path())
        }
        CaseKind::Script { invocation } => {
            let captured = run_invocation(invocation, project.path(), &env, &project_str)?;
            judge(&case.expect, &script_observed(&captured), project.path())
        }
        CaseKind::Flow { steps } => run_flow(
            steps,
            &case.expect,
            fixtures_root,
            project.path(),
            &env,
            &project_str,
        )?,
    };

    Ok(CaseOutcome {
        name: case.name.to_string(),
        verdict,
    })
}

/// The stdin a hook case sends: default payload ⊕ overlay, or the raw override.
fn stdin_for(
    event: HookEvent,
    payload: Option<&serde_json::Value>,
    payload_raw: Option<&str>,
    project: &str,
) -> String {
    if let Some(raw) = payload_raw {
        return raw.to_owned();
    }
    let mut value = default_payload(event);
    if let Some(overlay) = payload {
        merge(&mut value, overlay);
    }
    substitute_project(&mut value, project);
    value.to_string()
}

/// Runs one script invocation with `{project}` substituted in argv and env.
#[expect(
    clippy::literal_string_with_formatting_args,
    reason = "the `{project}` literal is a placeholder token replaced by str::replace, not a format string"
)]
fn run_invocation(
    invocation: &Invocation,
    cwd: &Path,
    env: &std::collections::BTreeMap<String, String>,
    project: &str,
) -> Result<crate::harness::Captured> {
    let argv: Vec<String> = invocation
        .argv
        .iter()
        .map(|a| a.replace("{project}", project))
        .collect();
    let mut child_env = env.clone();
    for (key, value) in &invocation.env {
        child_env.insert(key.clone(), value.replace("{project}", project));
    }
    run(&argv, cwd, &child_env, None, DEFAULT_TIMEOUT)
}

/// Scripts have no event semantics; the observation is the raw capture.
fn script_observed(captured: &crate::harness::Captured) -> Observed {
    Observed {
        exit: captured.exit,
        stdout: captured.stdout.clone(),
        stderr: captured.stderr.clone(),
        timed_out: captured.timed_out,
        ..Observed::default()
    }
}

/// Runs flow steps in one shared project; top-level expect judged after the last.
fn run_flow(
    steps: &[crate::case::Step],
    expect: &Expectations,
    fixtures_root: &Path,
    project_path: &Path,
    env: &std::collections::BTreeMap<String, String>,
    project_str: &str,
) -> Result<Verdict> {
    // The shared project was already materialized by the caller; steps mutate it.
    let mut last_observed: Option<Observed> = None;
    for (index, step) in steps.iter().enumerate() {
        if let Some(fixture) = &step.apply_fixture {
            overlay_into(fixtures_root, &fixture.0, project_path)?;
            continue;
        }
        let invocation = step
            .run
            .as_ref()
            .unwrap_or_else(|| unreachable!("validated in Case::from_raw"));
        let captured = run_invocation(invocation, project_path, env, project_str)?;
        let observed = script_observed(&captured);
        let default_expect = Expectations::default();
        if let Verdict::Fail(mismatches) = judge(
            step.expect.as_ref().unwrap_or(&default_expect),
            &observed,
            project_path,
        ) {
            return Ok(Verdict::Fail(
                mismatches
                    .into_iter()
                    .map(|m| format!("step {index}: {m}"))
                    .collect(),
            ));
        }
        last_observed = Some(observed);
    }
    // A flow with no run steps at all (fixture overlays only) has nothing to
    // observe. `files_exist` still holds — it only inspects the project tree
    // — but any expectation that needs an actual run cannot pass vacuously
    // against a fabricated `Observed::default()`.
    match last_observed {
        Some(observed) => Ok(judge(expect, &observed, project_path)),
        None if expect.expects_a_run() => Ok(Verdict::Fail(vec![String::from(
            "flow: no run step executed (only `apply_fixture` steps ran); \
             `expect` fields other than `files_exist` cannot be judged",
        )])),
        None => Ok(judge(expect, &Observed::default(), project_path)),
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]
    #![expect(
        clippy::panic,
        reason = "let-else diagnostics in tests panic by design"
    )]

    use super::{SuiteOptions, run_suite};

    /// A throwaway plugin: one `PreToolUse` gate hook + cases.
    fn plugin() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures/edits")).unwrap();
        std::fs::write(dir.path().join("tests/fixtures/edits/new.md"), "x").unwrap();
        std::fs::write(
            dir.path().join("hooks/hooks.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Write","hooks":[{"type":"command","command":"sh \"${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh\""}]}]}}"#,
        )
        .unwrap();
        // Deny writes that mention a lockfile; stay silent otherwise.
        std::fs::write(
            dir.path().join("hooks/gate.sh"),
            "payload=$(cat)\ncase \"$payload\" in\n  *Cargo.lock*) echo 'blocked: lockfile' >&2; exit 2 ;;\n  *) exit 0 ;;\nesac\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("tests/blocks-lockfile.yaml"),
            "event: PreToolUse\npayload:\n  tool_input:\n    file_path: Cargo.lock\nexpect:\n  decision: deny\n  stderr_contains: lockfile\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("tests/allows-clean.yaml"),
            "event: PreToolUse\nexpect:\n  output: none\n  exit: 0\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("tests/flow-writes.yaml"),
            "steps:\n  - run:\n      argv: [sh, -c, \"echo made > out.txt\"]\n    expect:\n      exit: 0\n  - apply_fixture: edits\nexpect:\n  files_exist: [out.txt, new.md]\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn the_three_case_kinds_run_green_against_the_gate_plugin() {
        let dir = plugin();
        let report = run_suite(dir.path(), &SuiteOptions::default()).unwrap();
        assert_eq!(report.outcomes.len(), 3, "{report:?}");
        assert!(report.all_green(), "{report:?}");
    }

    #[test]
    fn a_wrong_expectation_fails_that_case_and_only_that_case() {
        let dir = plugin();
        std::fs::write(
            dir.path().join("tests/wrong.yaml"),
            "event: PreToolUse\npayload:\n  tool_input:\n    file_path: Cargo.lock\nexpect:\n  decision: allow\n",
        )
        .unwrap();
        let report = run_suite(dir.path(), &SuiteOptions::default()).unwrap();
        assert!(!report.all_green());
        let failed: Vec<_> = report
            .outcomes
            .iter()
            .filter(|o| !matches!(o.verdict, crate::harness::Verdict::Pass))
            .collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].name, "wrong");
    }

    #[test]
    fn a_flows_top_level_expect_is_judged_against_the_last_steps_observation() {
        // The top-level `expect` must see what the last step actually did,
        // not a fabricated `Observed::default()` (exit 0, empty stdout).
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests")).unwrap();
        std::fs::write(
            dir.path().join("tests/flow-lies.yaml"),
            "steps:\n  - run:\n      argv: [sh, -c, \"echo hi; exit 7\"]\nexpect:\n  exit: 0\n  stdout_contains: hi\n",
        )
        .unwrap();
        let report = run_suite(dir.path(), &SuiteOptions::default()).unwrap();
        assert_eq!(report.outcomes.len(), 1);
        let crate::harness::Verdict::Fail(mismatches) = &report.outcomes[0].verdict else {
            panic!("exit: expected 0, got 7 must fail this case, {report:?}");
        };
        assert!(
            mismatches.iter().any(|m| m.contains("exit")),
            "{mismatches:?}"
        );
        assert!(
            !mismatches.iter().any(|m| m.contains("stdout")),
            "stdout_contains `hi` should pass against a step that printed it: {mismatches:?}"
        );
    }

    #[test]
    fn a_flow_with_no_run_step_fails_a_non_files_exist_expectation_instead_of_passing_vacuously() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures/edits")).unwrap();
        std::fs::write(dir.path().join("tests/fixtures/edits/new.md"), "x").unwrap();
        std::fs::write(
            dir.path().join("tests/flow-no-run.yaml"),
            "steps:\n  - apply_fixture: edits\nexpect:\n  exit: 0\n",
        )
        .unwrap();
        let report = run_suite(dir.path(), &SuiteOptions::default()).unwrap();
        assert_eq!(report.outcomes.len(), 1);
        let crate::harness::Verdict::Fail(mismatches) = &report.outcomes[0].verdict else {
            panic!("a flow with no run step must not pass `exit: 0` vacuously, {report:?}");
        };
        assert!(!mismatches.is_empty());
    }

    #[test]
    fn a_flow_with_no_run_step_still_judges_files_exist() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures/edits")).unwrap();
        std::fs::write(dir.path().join("tests/fixtures/edits/new.md"), "x").unwrap();
        std::fs::write(
            dir.path().join("tests/flow-no-run-files.yaml"),
            "steps:\n  - apply_fixture: edits\nexpect:\n  files_exist: [new.md]\n",
        )
        .unwrap();
        let report = run_suite(dir.path(), &SuiteOptions::default()).unwrap();
        assert_eq!(report.outcomes.len(), 1);
        assert!(report.all_green(), "{report:?}");
    }

    #[test]
    fn the_case_filter_narrows_the_run() {
        let dir = plugin();
        let report = run_suite(
            dir.path(),
            &SuiteOptions {
                case_filter: Some(String::from("blocks")),
            },
        )
        .unwrap();
        assert_eq!(report.outcomes.len(), 1);
    }
}
