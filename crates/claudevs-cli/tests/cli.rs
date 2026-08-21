//! Binary smoke tests: the exit-code contract, asserted through the real binary.

#![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]
#![expect(
    clippy::expect_used,
    reason = "tests expect() known-valid subprocess output"
)]

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
    // `check`'s first stage delegates to `claude plugin validate`, which
    // rejects a directory with no manifest at all; without one, the
    // pipeline never reaches wiring on a machine that has the binary.
    std::fs::create_dir_all(dir.path().join(".claude-plugin")).unwrap();
    std::fs::write(
        dir.path().join(".claude-plugin/plugin.json"),
        r#"{"name":"green-plugin","version":"0.1.0","description":"claudevs cli test fixture","author":{"name":"rstlix0x0"}}"#,
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

#[test]
fn check_exits_one_when_wiring_fails() {
    let dir = green_plugin();
    std::fs::write(
        dir.path().join("hooks/hooks.json"),
        r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"sh \"${CLAUDE_PLUGIN_ROOT}/../escape.sh\""}]}]}}"#,
    )
    .unwrap();
    let output = claudevs().args(["check"]).arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8_lossy(&output.stdout);
    // Anchored to the exact two-space indent `render_check_human` emits, so an
    // indentation regression fails here too rather than only in the render
    // unit test and the Makefile lane.
    assert!(text.contains("  FAIL  wiring\n"), "{text}");
}

#[test]
fn check_emits_machine_readable_stages_under_json() {
    let dir = green_plugin();
    let output = claudevs()
        .args(["check", "--json"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("check --json must emit JSON");
    let context = format!("{value}");
    let stages = value["stages"].as_array().expect(&context);
    assert!(!stages.is_empty(), "{value}");
    let first = &stages[0];
    assert!(first["name"].is_string(), "{value}");
    let status = first["status"].as_str().expect(&context);
    assert!(matches!(status, "passed" | "failed" | "skipped"), "{value}");
    assert!(first["detail"].is_string(), "{value}");
}

#[test]
fn doctor_reports_gaps_as_human_text_and_exits_one_when_a_probe_fails() {
    // `green_plugin()` builds a tempdir with no marketplace above it, so the
    // marketplace and install-layout probes report gaps.
    let dir = green_plugin();
    let output = claudevs()
        .args(["doctor"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("gap"), "{text}");
}

#[test]
fn doctor_json_emits_a_machine_readable_probe_list() {
    let dir = green_plugin();
    let output = claudevs()
        .args(["doctor", "--json"])
        .arg(dir.path())
        .output()
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("doctor --json must emit JSON");
    let context = format!("{value}");
    let probes = value["probes"].as_array().expect(&context);
    assert!(!probes.is_empty(), "{value}");
    let first = &probes[0];
    assert!(first["name"].is_string(), "{value}");
    let status = first["status"].as_str().expect(&context);
    assert!(matches!(status, "ok" | "warning" | "gap"), "{value}");
    assert!(first["detail"].is_string(), "{value}");
}
