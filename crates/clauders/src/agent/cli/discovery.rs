//! Locating and version-gating the backend binary.

use std::path::{Path, PathBuf};

use crate::agent::error::AgentError;
use crate::agent::options::Options;

/// Lowest backend version the SDK is validated against.
const MIN_VERSION: (u32, u32, u32) = (2, 0, 0);
/// Name searched for on `PATH` and in fallback locations.
const BINARY_NAME: &str = "claude";

/// Resolve the backend binary path from options and the ambient environment.
///
/// Resolution order: explicit override, then each `PATH` directory, then the
/// per-user fallback install location.
///
/// # Errors
/// Returns [`AgentError::BinaryNotFound`] (carrying every path inspected) if
/// no binary is located.
pub(super) fn discover(options: &Options) -> Result<PathBuf, AgentError> {
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    discover_in(options, &path_dirs, home.as_deref())
}

/// Environment-injected core of [`discover`], for deterministic testing.
fn discover_in(
    options: &Options,
    path_dirs: &[PathBuf],
    home: Option<&Path>,
) -> Result<PathBuf, AgentError> {
    let mut searched = Vec::new();
    if let Some(p) = &options.path_to_executable {
        searched.push(p.clone());
        if p.exists() {
            return Ok(p.clone());
        }
        return Err(AgentError::BinaryNotFound { searched });
    }
    for dir in path_dirs {
        let cand = dir.join(BINARY_NAME);
        let exists = cand.is_file();
        searched.push(cand.clone());
        if exists {
            return Ok(cand);
        }
    }
    if let Some(home) = home {
        let cand = home.join(".claude").join("local").join(BINARY_NAME);
        let exists = cand.is_file();
        searched.push(cand.clone());
        if exists {
            return Ok(cand);
        }
    }
    Err(AgentError::BinaryNotFound { searched })
}

/// Gate a reported version string against the supported minimum.
///
/// Below the minimum: a `tracing::warn!` by default, promoted to a hard
/// [`AgentError::BinaryVersionUnsupported`] when `require_min` is set. An
/// unparseable string warns and is allowed (forward-compat).
///
/// # Errors
/// Returns [`AgentError::BinaryVersionUnsupported`] when the version is below
/// the minimum and `require_min` is `true`.
pub(super) fn check_version(found: &str, require_min: bool) -> Result<(), AgentError> {
    match parse_semver(found) {
        Some(version) if version >= MIN_VERSION => Ok(()),
        Some(_) if require_min => Err(AgentError::BinaryVersionUnsupported {
            found: found.to_string(),
            minimum: "2.0.0".to_string(),
        }),
        Some(_) => {
            tracing::warn!(
                found,
                "backend binary is older than the recommended minimum 2.0.0"
            );
            Ok(())
        }
        None => {
            tracing::warn!(found, "could not parse backend binary version");
            Ok(())
        }
    }
}

/// Parse a `major.minor.patch` triple from the first dotted token.
fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let token = s.split_whitespace().find(|t| t.contains('.'))?;
    let mut parts = token.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().map_or(0, |p| {
        p.trim_matches(|c: char| !c.is_ascii_digit())
            .parse()
            .unwrap_or(0)
    });
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(
        clippy::panic,
        reason = "test exhaustive match arms use panic for context"
    )]

    use super::{check_version, discover_in, parse_semver};
    use crate::agent::error::AgentError;
    use crate::agent::options::Options;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn override_path_that_exists_is_returned() {
        let dir = std::env::temp_dir().join(format!("clauders-disc-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("mkdir");
        let bin = dir.join("claude");
        fs::write(&bin, b"#!/bin/sh\n").expect("write");
        let opts = Options::builder().path_to_executable(bin.clone()).build();
        let found = discover_in(&opts, &[], None).expect("discover");
        assert_eq!(found, bin);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_override_errors_with_searched_path() {
        let opts = Options::builder()
            .path_to_executable(PathBuf::from("/no/such/claude"))
            .build();
        let err = discover_in(&opts, &[], None).expect_err("should fail");
        match err {
            AgentError::BinaryNotFound { searched } => {
                assert!(searched.iter().any(|p| p.ends_with("claude")));
            }
            other => panic!("expected BinaryNotFound, got {other:?}"),
        }
    }

    #[test]
    fn found_on_path_dirs() {
        let dir = std::env::temp_dir().join(format!("clauders-path-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("mkdir");
        let bin = dir.join("claude");
        fs::write(&bin, b"#!/bin/sh\n").expect("write");
        let opts = Options::default();
        let found = discover_in(&opts, std::slice::from_ref(&dir), None).expect("discover");
        assert_eq!(found, bin);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_version_token() {
        assert_eq!(parse_semver("2.1.3 (Claude Code)"), Some((2, 1, 3)));
        assert_eq!(parse_semver("1.9.0"), Some((1, 9, 0)));
        assert_eq!(parse_semver("garbage"), None);
    }

    #[test]
    fn version_below_minimum_is_warn_unless_required() {
        assert!(check_version("1.5.0", false).is_ok());
        let err = check_version("1.5.0", true).expect_err("should fail");
        assert!(matches!(err, AgentError::BinaryVersionUnsupported { .. }));
        assert!(check_version("2.0.0", true).is_ok());
    }
}
