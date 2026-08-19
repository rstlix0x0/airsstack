//! Running one discovered Lua case file end to end.
//!
//! A sibling of the case model rather than folded into [`super::lua`] because
//! it reaches into [`crate::suite`] for [`crate::suite::run_case`] and
//! [`crate::suite::CaseOutcome`] — the model stays free of the runner's
//! dependency on the suite layer.

use crate::error::Result;
use crate::suite::{CaseOutcome, SuiteOptions, selected};

/// Runs one Lua case file: data cases and scripted cases alike.
///
/// # Errors
///
/// Same conditions as [`crate::suite::run_suite`].
pub fn run_lua_file(
    plugin_dir: &std::path::Path,
    fixtures_root: &std::path::Path,
    path: &std::path::Path,
    options: &SuiteOptions,
) -> Result<Vec<CaseOutcome>> {
    let loaded = super::lua::load(super::lua::engine_for(plugin_dir, fixtures_root)?, path)?;
    let mut outcomes = Vec::new();
    for case in &loaded.cases {
        if selected(options, case.name.as_str()) {
            outcomes.push(crate::suite::run_case(plugin_dir, fixtures_root, case)?);
        }
    }
    // Filter *before* execution: a scripted case excluded by `--case` must
    // not run at all, side effects included, not merely go unreported.
    let selected_scripted: Vec<_> = loaded
        .scripted
        .iter()
        .filter(|name| selected(options, name.as_str()))
        .cloned()
        .collect();
    for (name, outcome) in super::lua::run_scripted(&loaded, &selected_scripted)? {
        outcomes.push(CaseOutcome {
            name: name.to_string(),
            verdict: match outcome {
                Ok(()) => crate::harness::Verdict::Pass,
                Err(reason) => crate::harness::Verdict::Fail(vec![reason]),
            },
        });
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::run_lua_file;
    use crate::suite::SuiteOptions;

    /// One Lua file with a data case and a scripted case, both trivially green.
    fn plugin() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();
        std::fs::write(
            dir.path().join("tests/mixed_test.lua"),
            r#"return {
                 data_case = { invocation = { argv = { "true" } } },
                 scripted_case = function(t) end,
               }"#,
        )
        .unwrap();
        dir
    }

    fn run(dir: &tempfile::TempDir, options: &SuiteOptions) -> Vec<crate::suite::CaseOutcome> {
        run_lua_file(
            dir.path(),
            &dir.path().join("tests/fixtures"),
            &dir.path().join("tests/mixed_test.lua"),
            options,
        )
        .unwrap()
    }

    #[test]
    fn no_filter_dispatches_every_data_and_scripted_case() {
        let dir = plugin();
        let mut names: Vec<_> = run(&dir, &SuiteOptions::default())
            .into_iter()
            .map(|o| o.name)
            .collect();
        names.sort();
        assert_eq!(names, vec!["data_case", "scripted_case"]);
    }

    /// Like [`plugin`], but the scripted case leaves a marker file behind when
    /// it actually runs, so a filtered-out scripted case can be proven not to
    /// have executed rather than merely not reported.
    fn plugin_with_marker() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("tests/fixtures")).unwrap();
        std::fs::write(
            dir.path().join("tests/mixed_test.lua"),
            r#"return {
                 data_case = { invocation = { argv = { "true" } } },
                 scripted_case = function(t)
                   t.script({ "sh", "-c", "echo ran > MARKER" })
                 end,
               }"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn the_filter_selects_a_matching_data_case_and_excludes_the_scripted_one() {
        let dir = plugin_with_marker();
        let options = SuiteOptions {
            case_filter: Some(String::from("data_")),
        };
        let outcomes = run(&dir, &options);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].name, "data_case");
        assert!(
            !dir.path().join("MARKER").exists(),
            "the filtered-out scripted case must not have executed its side effect"
        );
    }

    #[test]
    fn the_filter_selects_a_matching_scripted_case_and_excludes_the_data_one() {
        let dir = plugin();
        let options = SuiteOptions {
            case_filter: Some(String::from("scripted_")),
        };
        let outcomes = run(&dir, &options);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].name, "scripted_case");
    }

    #[test]
    fn a_filter_matching_nothing_dispatches_nothing() {
        let dir = plugin();
        let options = SuiteOptions {
            case_filter: Some(String::from("nonexistent")),
        };
        assert!(run(&dir, &options).is_empty());
    }
}
