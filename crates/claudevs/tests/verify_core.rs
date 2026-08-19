//! Whole-engine integration over the committed fixture plugin.

#![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

use std::path::PathBuf;

use claudevs::{SuiteOptions, exit_code, render_human, run_suite};

fn fixture_plugin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal-plugin")
}

#[test]
fn the_exemplar_plugin_runs_fully_green() {
    let report = run_suite(&fixture_plugin(), &SuiteOptions::default()).unwrap();
    let rendered = render_human(&report);
    // 3 YAML + 2 generated + 1 scripted = 6 cases, plus 1 native suite.
    assert_eq!(report.outcomes.len(), 6, "{rendered}");
    assert_eq!(report.native.len(), 1, "{rendered}");
    assert!(report.all_green(), "{rendered}");
    assert_eq!(exit_code(&report), 0);
}

#[test]
fn the_filter_selects_across_yaml_and_lua_alike() {
    let report = run_suite(
        &fixture_plugin(),
        &SuiteOptions {
            case_filter: Some(String::from("blocks")),
        },
    )
    .unwrap();
    // blocks-lockfile (YAML) + blocks_Cargo_lock + blocks_poetry_lock (Lua data).
    assert_eq!(report.outcomes.len(), 3);
}
