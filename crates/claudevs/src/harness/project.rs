//! Materializing a fixture directory as the case's temp project.
//!
//! Fixtures are plain directories under `tests/fixtures/`. A fixture holding a
//! file named `.gitinit` gets `git init` + one commit (the marker itself is not
//! copied). Nothing ever executes against the plugin's real checkout.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// A materialized temp project (deleted on drop).
#[derive(Debug)]
pub struct Project {
    dir: tempfile::TempDir,
}

impl Project {
    /// An empty project.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] when the temp dir cannot be created.
    pub fn empty() -> Result<Self> {
        let dir = tempfile::tempdir().map_err(|source| Error::Io {
            operation: "create temp project",
            path: String::from("(tempdir)"),
            source,
        })?;
        Ok(Self { dir })
    }

    /// A project seeded from `fixtures_root/<name>`.
    ///
    /// # Errors
    ///
    /// [`Error::Fixture`] when the fixture is missing; [`Error::Io`] on copy failure.
    pub fn from_fixture(fixtures_root: &Path, name: &str) -> Result<Self> {
        let source = fixtures_root.join(name);
        if !source.is_dir() {
            return Err(Error::Fixture {
                name: name.to_owned(),
                reason: format!("no directory at `{}`", source.display()),
            });
        }
        let project = Self::empty()?;
        copy_tree(&source, project.path())?;

        if source.join(".gitinit").is_file() {
            let _ = std::fs::remove_file(project.path().join(".gitinit"));
            git(project.path(), &["init", "-q"])?;
            git(
                project.path(),
                &[
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "user.name=t",
                    "commit",
                    "-q",
                    "--allow-empty",
                    "-m",
                    "init",
                ],
            )?;
        }
        Ok(project)
    }

    /// Overlays `fixtures_root/<name>` onto this project (flow `apply_fixture`).
    ///
    /// # Errors
    ///
    /// Same conditions as [`Project::from_fixture`].
    pub fn overlay(&self, fixtures_root: &Path, name: &str) -> Result<()> {
        overlay_into(fixtures_root, name, self.path())
    }

    /// The project's root directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

/// Overlays `fixtures_root/<name>` into an existing directory.
///
/// # Errors
///
/// Same conditions as [`Project::from_fixture`].
pub fn overlay_into(fixtures_root: &Path, name: &str, into: &Path) -> Result<()> {
    let source = fixtures_root.join(name);
    if !source.is_dir() {
        return Err(Error::Fixture {
            name: name.to_owned(),
            reason: format!("no directory at `{}`", source.display()),
        });
    }
    copy_tree(&source, into)
}

/// Recursively copies `from` into the existing directory `to`.
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents that the installed layout shares this copier"
)]
pub(crate) fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(from).sort_by_file_name() {
        let entry = entry.map_err(|e| Error::Io {
            operation: "walk fixture",
            path: from.display().to_string(),
            source: e.into(),
        })?;
        let rel: PathBuf = entry
            .path()
            .strip_prefix(from)
            .unwrap_or_else(|_| entry.path())
            .to_path_buf();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let dest = to.join(&rel);
        let io = |source| Error::Io {
            operation: "copy fixture",
            path: dest.display().to_string(),
            source,
        };
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest).map_err(io)?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::copy(entry.path(), &dest).map_err(io)?;
        }
    }
    Ok(())
}

/// Runs git in `dir`, discarding output.
fn git(dir: &Path, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|source| Error::Io {
            operation: "run git",
            path: dir.display().to_string(),
            source,
        })?;
    if status.status.success() {
        Ok(())
    } else {
        Err(Error::Fixture {
            name: dir.display().to_string(),
            reason: format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&status.stderr)
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::Project;

    fn fixtures() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("repo/src")).unwrap();
        std::fs::write(root.path().join("repo/Cargo.toml"), "[package]\n").unwrap();
        std::fs::write(root.path().join("repo/src/main.rs"), "fn main() {}\n").unwrap();
        root
    }

    #[test]
    fn a_fixture_is_copied_into_a_fresh_temp_dir() {
        let root = fixtures();
        let project = Project::from_fixture(root.path(), "repo").unwrap();
        assert!(project.path().join("src/main.rs").is_file());
        assert_ne!(project.path(), root.path().join("repo"));
    }

    #[test]
    fn a_gitinit_marker_produces_a_repo_and_is_not_copied() {
        let root = fixtures();
        std::fs::write(root.path().join("repo/.gitinit"), "").unwrap();
        let project = Project::from_fixture(root.path(), "repo").unwrap();
        assert!(project.path().join(".git").is_dir());
        assert!(!project.path().join(".gitinit").exists());
    }

    #[test]
    fn a_missing_fixture_is_an_author_error_naming_it() {
        let root = fixtures();
        let error = Project::from_fixture(root.path(), "nope")
            .unwrap_err()
            .to_string();
        assert!(error.contains("nope"), "{error}");
    }

    #[test]
    fn overlay_adds_files_to_an_existing_project() {
        let root = fixtures();
        std::fs::create_dir_all(root.path().join("edits")).unwrap();
        std::fs::write(root.path().join("edits/new.md"), "x").unwrap();
        let project = Project::from_fixture(root.path(), "repo").unwrap();
        project.overlay(root.path(), "edits").unwrap();
        assert!(project.path().join("new.md").is_file());
        assert!(project.path().join("src/main.rs").is_file());
    }
}
