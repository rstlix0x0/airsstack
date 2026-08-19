//! Spawning one child: stdin feed, capture, poll-based timeout.
//!
//! Hook commands are shell strings in hooks.json (they interpolate
//! `${CLAUDE_PLUGIN_ROOT}`), so hooks spawn as `sh -c <command>`; script
//! invocations spawn their argv directly. A hung child becomes a verdict
//! failure, never a hung run.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::{Error, Result};

/// What a spawned child produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    /// Exit code (`-1` for signal death, `-2` for timeout kill).
    pub exit: i32,
    /// Raw stdout.
    pub stdout: String,
    /// Raw stderr.
    pub stderr: String,
    /// Whether the timeout fired.
    pub timed_out: bool,
}

/// Default per-case timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Runs `argv` in `cwd` with `env` added, feeding `stdin` when given.
///
/// # Errors
///
/// [`Error::Io`] when the child cannot be spawned at all (missing program).
pub fn run(
    argv: &[String],
    cwd: &Path,
    env: &BTreeMap<String, String>,
    stdin: Option<&str>,
    timeout: Duration,
) -> Result<Captured> {
    let (program, rest) = argv.split_first().ok_or_else(|| Error::Io {
        operation: "spawn",
        path: String::from("(empty argv)"),
        source: std::io::Error::other("the argv array is empty"),
    })?;

    let mut child = Command::new(program)
        .args(rest)
        .current_dir(cwd)
        .envs(env)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| Error::Io {
            operation: "spawn",
            path: program.clone(),
            source,
        })?;

    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        // A child that never reads sees EPIPE; ignoring the write error keeps
        // hostile-input cases (closed stdin) from failing the harness itself.
        let _ = pipe.write_all(text.as_bytes());
    }

    let deadline = Instant::now() + timeout;
    let timed_out = loop {
        match child.try_wait().map_err(|source| Error::Io {
            operation: "wait",
            path: program.clone(),
            source,
        })? {
            Some(_) => break false,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                break true;
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };

    let output = child.wait_with_output().map_err(|source| Error::Io {
        operation: "collect output",
        path: program.clone(),
        source,
    })?;

    Ok(Captured {
        exit: if timed_out {
            -2
        } else {
            output.status.code().unwrap_or(-1)
        },
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        timed_out,
    })
}

/// Runs a hooks.json shell command string (spawned as `sh -c`).
///
/// # Errors
///
/// Same conditions as [`run`].
pub fn run_shell(
    command: &str,
    cwd: &Path,
    env: &BTreeMap<String, String>,
    stdin: Option<&str>,
    timeout: Duration,
) -> Result<Captured> {
    let argv = vec![String::from("sh"), String::from("-c"), command.to_owned()];
    run(&argv, cwd, env, stdin, timeout)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]

    use super::{DEFAULT_TIMEOUT, run, run_shell};
    use std::collections::BTreeMap;
    use std::time::Duration;

    fn cwd() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn stdout_stderr_and_exit_are_captured() {
        let dir = cwd();
        let captured = run_shell(
            "echo out; echo err >&2; exit 3",
            dir.path(),
            &BTreeMap::new(),
            None,
            DEFAULT_TIMEOUT,
        )
        .unwrap();
        assert_eq!(captured.exit, 3);
        assert_eq!(captured.stdout.trim(), "out");
        assert_eq!(captured.stderr.trim(), "err");
    }

    #[test]
    fn stdin_reaches_the_child_and_env_is_applied() {
        let dir = cwd();
        let env = BTreeMap::from([(String::from("CLAUDE_PLUGIN_ROOT"), String::from("/pr"))]);
        let captured = run_shell(
            "read line; echo \"$line-$CLAUDE_PLUGIN_ROOT\"",
            dir.path(),
            &env,
            Some("payload\n"),
            DEFAULT_TIMEOUT,
        )
        .unwrap();
        assert_eq!(captured.stdout.trim(), "payload-/pr");
    }

    #[test]
    fn a_hung_child_is_killed_and_reported_as_timeout() {
        let dir = cwd();
        let captured = run_shell(
            "sleep 30",
            dir.path(),
            &BTreeMap::new(),
            None,
            Duration::from_millis(200),
        )
        .unwrap();
        assert!(captured.timed_out);
        assert_eq!(captured.exit, -2);
    }

    #[test]
    fn a_missing_program_is_a_spawn_error_not_a_verdict() {
        let dir = cwd();
        assert!(
            run(
                &[String::from("claudevs-definitely-not-installed")],
                dir.path(),
                &BTreeMap::new(),
                None,
                DEFAULT_TIMEOUT,
            )
            .is_err()
        );
    }
}
