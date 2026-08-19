//! The environment a spawned hook or script sees.
//!
//! `CLAUDE_PLUGIN_ROOT` is the one variable verified load-bearing in this
//! repository's hooks (every hooks.json command interpolates it). Further
//! runtime variables are added here as the P1 execution pins them against the
//! hook documentation; cases can always add their own via `env`.

use std::collections::BTreeMap;
use std::path::Path;

/// The base environment for a case's children.
#[must_use]
pub fn base_env(plugin_root: &Path, project: &Path) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            String::from("CLAUDE_PLUGIN_ROOT"),
            plugin_root.display().to_string(),
        ),
        (
            String::from("CLAUDE_PROJECT_DIR"),
            project.display().to_string(),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::base_env;
    use std::path::Path;

    #[test]
    fn plugin_root_and_project_dir_are_set() {
        let env = base_env(Path::new("/p/plugin"), Path::new("/tmp/proj"));
        assert_eq!(env["CLAUDE_PLUGIN_ROOT"], "/p/plugin");
        assert_eq!(env["CLAUDE_PROJECT_DIR"], "/tmp/proj");
    }
}
