//! Session configuration for the Agent SDK.

use std::path::PathBuf;
use std::time::Duration;

use crate::agent::permissions::PermissionMode;
use crate::agent::types::McpServerConfig;
use crate::types::ModelId;

/// Default graceful-shutdown window before the supervisor forces a kill.
const DEFAULT_SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// Configuration for a `Client` / `query` session.
///
/// Built via [`Options::builder`]. Carries everything the runtime needs to
/// discover, spawn, and configure the binary. In-loop handler fields (hooks,
/// permission policy) are not yet supported.
#[derive(Clone, Debug)]
pub struct Options {
    /// Optional system prompt forwarded in the initialize handshake.
    pub system_prompt: Option<String>,
    /// Model override.
    pub model: Option<ModelId>,
    /// Tool-gating mode.
    pub permission_mode: PermissionMode,
    /// Tool allowlist forwarded to the binary.
    pub allowed_tools: Vec<String>,
    /// Tool denylist forwarded to the binary.
    pub disallowed_tools: Vec<String>,
    /// External MCP servers (opaque pass-through).
    pub mcp_servers: Vec<McpServerConfig>,
    /// Working directory for the subprocess.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables for the subprocess.
    pub env: Vec<(String, String)>,
    /// Turn cap forwarded to the binary.
    pub max_turns: Option<u32>,
    /// Override for binary discovery.
    pub path_to_executable: Option<PathBuf>,
    /// Extra args prepended to the SDK-managed argv.
    pub executable_args: Vec<String>,
    /// Promote a too-old binary from a warning to a hard error.
    pub require_min_version: bool,
    /// Graceful-exit window before a forced kill.
    pub shutdown_grace: Duration,
}

impl Options {
    /// Start building an `Options` with defaults.
    #[must_use]
    pub fn builder() -> OptionsBuilder {
        OptionsBuilder::default()
    }
}

impl Default for Options {
    fn default() -> Self {
        OptionsBuilder::default().build()
    }
}

/// Builder for [`Options`].
#[derive(Clone, Debug, Default)]
pub struct OptionsBuilder {
    system_prompt: Option<String>,
    model: Option<ModelId>,
    permission_mode: PermissionMode,
    allowed_tools: Vec<String>,
    disallowed_tools: Vec<String>,
    mcp_servers: Vec<McpServerConfig>,
    cwd: Option<PathBuf>,
    env: Vec<(String, String)>,
    max_turns: Option<u32>,
    path_to_executable: Option<PathBuf>,
    executable_args: Vec<String>,
    require_min_version: bool,
    shutdown_grace: Option<Duration>,
}

impl OptionsBuilder {
    /// Set the system prompt.
    #[must_use]
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model.
    #[must_use]
    pub fn model(mut self, model: ModelId) -> Self {
        self.model = Some(model);
        self
    }

    /// Set the permission mode.
    #[must_use]
    pub const fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Set the tool allowlist.
    #[must_use]
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Set the tool denylist.
    #[must_use]
    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.disallowed_tools = tools;
        self
    }

    /// Set the external MCP servers.
    #[must_use]
    pub fn mcp_servers(mut self, servers: Vec<McpServerConfig>) -> Self {
        self.mcp_servers = servers;
        self
    }

    /// Set the subprocess working directory.
    #[must_use]
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Append an environment variable for the subprocess.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set the turn cap.
    #[must_use]
    pub const fn max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// Override the binary path.
    #[must_use]
    pub fn path_to_executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.path_to_executable = Some(path.into());
        self
    }

    /// Prepend extra args to the SDK-managed argv.
    #[must_use]
    pub fn executable_args(mut self, args: Vec<String>) -> Self {
        self.executable_args = args;
        self
    }

    /// Require a minimum binary version (hard error if too old).
    #[must_use]
    pub const fn require_min_version(mut self, require: bool) -> Self {
        self.require_min_version = require;
        self
    }

    /// Override the graceful-shutdown window.
    #[must_use]
    pub const fn shutdown_grace(mut self, grace: Duration) -> Self {
        self.shutdown_grace = Some(grace);
        self
    }

    /// Finalize into an [`Options`].
    #[must_use]
    pub fn build(self) -> Options {
        Options {
            system_prompt: self.system_prompt,
            model: self.model,
            permission_mode: self.permission_mode,
            allowed_tools: self.allowed_tools,
            disallowed_tools: self.disallowed_tools,
            mcp_servers: self.mcp_servers,
            cwd: self.cwd,
            env: self.env,
            max_turns: self.max_turns,
            path_to_executable: self.path_to_executable,
            executable_args: self.executable_args,
            require_min_version: self.require_min_version,
            shutdown_grace: self.shutdown_grace.unwrap_or(DEFAULT_SHUTDOWN_GRACE),
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::time::Duration;

    use super::Options;
    use crate::agent::permissions::PermissionMode;
    use crate::types::ModelId;

    #[test]
    fn defaults_are_sane() {
        let opts = Options::builder().build();
        assert_eq!(opts.permission_mode, PermissionMode::Default);
        assert_eq!(opts.shutdown_grace, Duration::from_secs(5));
        assert!(!opts.require_min_version);
        assert!(opts.model.is_none());
        assert!(opts.allowed_tools.is_empty());
    }

    #[test]
    fn builder_sets_fields() {
        let opts = Options::builder()
            .model(ModelId::custom("claude-sonnet-4-5").expect("model"))
            .permission_mode(PermissionMode::AcceptEdits)
            .allowed_tools(vec!["Bash".to_string()])
            .max_turns(7)
            .shutdown_grace(Duration::from_secs(2))
            .build();
        assert_eq!(opts.model.as_ref().map(ModelId::as_str), Some("claude-sonnet-4-5"));
        assert_eq!(opts.permission_mode, PermissionMode::AcceptEdits);
        assert_eq!(opts.allowed_tools, vec!["Bash".to_string()]);
        assert_eq!(opts.max_turns, Some(7));
        assert_eq!(opts.shutdown_grace, Duration::from_secs(2));
    }
}
