//! The same cases, run a second time against the simulated install layout.

#![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

use std::path::PathBuf;

use claudevs::{SuiteOptions, run_suite_installed};

fn fixture_plugin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal-plugin")
}

/// Copies the exemplar under a marketplace root this test owns.
///
/// No fixture ships its own `marketplace.json`, so a plugin used in place is
/// keyed by whatever ancestor happens to hold one — this repository's own root.
/// Owning the root keeps the install layout a property of the test rather than
/// of where the crate is checked out.
fn exemplar_under_its_own_marketplace() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join(".claude-plugin")).unwrap();
    std::fs::write(
        root.path().join(".claude-plugin/marketplace.json"),
        r#"{"name":"airsstack","plugins":[]}"#,
    )
    .unwrap();
    let plugin = root.path().join("plugins/minimal-plugin");
    std::fs::create_dir_all(&plugin).unwrap();
    let status = std::process::Command::new("cp")
        .args(["-R", &format!("{}/.", fixture_plugin().display())])
        .arg(&plugin)
        .status()
        .unwrap();
    assert!(status.success());
    (root, plugin)
}

#[test]
fn every_case_passes_again_from_the_install_cache() {
    let (_root, plugin) = exemplar_under_its_own_marketplace();
    let report = run_suite_installed(&plugin, &SuiteOptions::default()).unwrap();
    assert_eq!(report.outcomes.len(), 7, "{report:?}");
    assert!(report.all_green(), "{report:?}");
}

#[test]
fn a_hook_that_only_resolves_in_the_checkout_fails_from_the_cache() {
    // A command reaching outside CLAUDE_PLUGIN_ROOT works in the source tree and
    // cannot work once installed; that difference is the whole point of the
    // second context.
    let source = fixture_plugin();
    let temporary = tempfile::tempdir().unwrap();
    let plugin = temporary.path().join("plugins/leaky");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::create_dir_all(temporary.path().join(".claude-plugin")).unwrap();
    std::fs::write(
        temporary.path().join(".claude-plugin/marketplace.json"),
        r#"{"name":"airsstack","plugins":[]}"#,
    )
    .unwrap();
    // Copy the exemplar, then point one hook at a path that exists only beside
    // the checkout. `cp -R` rather than the engine's own copier because
    // `harness::copy_tree` is pub(crate) and an integration test links against
    // the public API only.
    let status = std::process::Command::new("cp")
        .args(["-R", &format!("{}/.", source.display())])
        .arg(&plugin)
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::write(temporary.path().join("plugins/outside.sh"), "exit 0\n").unwrap();
    std::fs::write(
        plugin.join("hooks/hooks.json"),
        r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"sh \"${CLAUDE_PLUGIN_ROOT}/../outside.sh\" && echo 'reminder: load the snapshot'"}]}]}}"#,
    )
    .unwrap();

    let source_run = claudevs::run_suite(
        &plugin,
        &SuiteOptions {
            case_filter: Some(String::from("session-banner")),
        },
    )
    .unwrap();
    assert!(source_run.all_green(), "{source_run:?}");

    let installed_run = run_suite_installed(
        &plugin,
        &SuiteOptions {
            case_filter: Some(String::from("session-banner")),
        },
    )
    .unwrap();
    assert!(
        !installed_run.all_green(),
        "the leaky hook must fail once installed: {installed_run:?}"
    );
}
