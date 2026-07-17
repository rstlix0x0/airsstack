use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::process::Command;

/// Per-chunk stderr callback: invoked with one valid UTF-8 chunk at a time.
type StderrCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Declarative configuration for spawning a managed child process.
#[derive(Clone)]
pub struct ProcessConfig {
    /// Executable to run.
    pub program: PathBuf,
    /// Arguments passed to the executable.
    pub args: Vec<String>,
    /// Working directory; inherits the parent's when `None`.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables applied on top of the inherited set.
    pub env: Vec<(String, String)>,
    /// How long graceful shutdown waits before escalating to a kill.
    pub shutdown_grace: Duration,
    /// Cap on bytes buffered per stdout line before erroring (`None` =
    /// unbounded).
    pub max_buffer_size: Option<std::num::NonZeroUsize>,
    /// Per-chunk stderr callback forwarded to the stderr drain.
    pub stderr_callback: Option<StderrCallback>,
}

impl ProcessConfig {
    /// Create a config for `program` with empty args/env and a 5s grace.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            shutdown_grace: Duration::from_secs(5),
            max_buffer_size: None,
            stderr_callback: None,
        }
    }
}

impl std::fmt::Debug for ProcessConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessConfig")
            .field("program", &self.program)
            .field("args", &self.args)
            .field("cwd", &self.cwd)
            .field("env", &self.env)
            .field("shutdown_grace", &self.shutdown_grace)
            .field("max_buffer_size", &self.max_buffer_size)
            .field("stderr_callback", &self.stderr_callback.is_some())
            .finish()
    }
}

/// Build a `tokio` `Command` from a config: all three stdio streams are
/// piped, `kill_on_drop` is enabled as a last-resort safety net, and on
/// Unix the child becomes the leader of a new process group so the whole
/// group (including the child's own descendants) can be signalled at once.
pub(super) fn build_command(cfg: &ProcessConfig) -> Command {
    let mut cmd = Command::new(&cfg.program);
    cmd.args(&cfg.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(cwd) = &cfg.cwd {
        cmd.current_dir(cwd);
    }
    for (key, value) in &cfg.env {
        cmd.env(key, value);
    }
    #[cfg(unix)]
    {
        // Safe std API: child leads a new group; pgid == child pid.
        cmd.process_group(0);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn process_config_new_sets_five_second_grace_and_empty_args() {
        let cfg = ProcessConfig::new("/bin/echo");
        assert_eq!(cfg.program.as_os_str(), "/bin/echo");
        assert!(cfg.args.is_empty());
        assert!(cfg.cwd.is_none());
        assert!(cfg.env.is_empty());
        assert_eq!(cfg.shutdown_grace, Duration::from_secs(5));
    }
}
