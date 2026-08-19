//! Binary smoke tests: the spec §3 exit-code contract.

#![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

use std::process::Command;

fn claudevs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_claudevs"))
}

/// A tiny green plugin: one `SessionStart` echo hook, one YAML case.
fn green_plugin() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(
        dir.path().join("hooks/hooks.json"),
        r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo hello-banner"}]}]}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("tests/banner.yaml"),
        "event: SessionStart\nexpect:\n  context_contains: hello-banner\n",
    )
    .unwrap();
    dir
}

#[test]
fn a_green_suite_exits_zero() {
    let dir = green_plugin();
    let output = claudevs().args(["test"]).arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn a_failing_case_exits_one() {
    let dir = green_plugin();
    std::fs::write(
        dir.path().join("tests/wrong.yaml"),
        "event: SessionStart\nexpect:\n  context_contains: absent-text\n",
    )
    .unwrap();
    let output = claudevs().args(["test"]).arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn a_relative_plugin_path_still_resolves_plugin_root_for_hooks() {
    // Hook commands interpolate ${CLAUDE_PLUGIN_ROOT} but run inside the temp
    // project, so the harness must hand children an absolute plugin root even
    // when the CLI was given a relative path (as `claudevs test` defaults to).
    let dir = tempfile::tempdir().unwrap();
    let plugin = dir.path().join("my-plugin");
    std::fs::create_dir_all(plugin.join("hooks")).unwrap();
    std::fs::create_dir_all(plugin.join("tests")).unwrap();
    std::fs::write(
        plugin.join("hooks/hooks.json"),
        r#"{"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"sh \"${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh\""}]}]}}"#,
    )
    .unwrap();
    std::fs::write(plugin.join("hooks/gate.sh"), "echo 'blocked' >&2; exit 2\n").unwrap();
    std::fs::write(
        plugin.join("tests/denies.yaml"),
        "event: PreToolUse\nexpect:\n  decision: deny\n",
    )
    .unwrap();
    let output = claudevs()
        .args(["test", "my-plugin"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn an_unrunnable_suite_exits_two() {
    let dir = tempfile::tempdir().unwrap(); // no tests/ at all
    let output = claudevs().args(["test"]).arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn migrate_prints_lua_and_exits_zero() {
    let dir = green_plugin();
    let output = claudevs()
        .args(["migrate"])
        .arg(dir.path().join("tests/banner.yaml"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("return {"));
}
