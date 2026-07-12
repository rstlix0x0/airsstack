//! Session configuration for the Agent SDK.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::agent::capabilities::HookEvent;
use crate::agent::hooks::{Hook, HookRegistry};
use crate::agent::mcp::{SdkMcpRegistry, SdkMcpServer};
use crate::agent::permissions::{PermissionJudge, PermissionMode, PermissionPolicy};
use crate::agent::subagents::AgentDefinition;
use crate::agent::system_prompt::SystemPromptConfig;
use crate::agent::types::{McpServerConfig, SessionControl};
use crate::messages::structured_outputs::OutputConfig;
use crate::types::{MaxTokens, ModelId};

/// Default graceful-shutdown window before the supervisor forces a kill.
const DEFAULT_SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// Default per-request output-token ceiling when the caller sets none.
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Configuration for a `Client` / `query` session.
///
/// Built via [`Options::builder`]. Carries everything the runtime needs to
/// discover, spawn, and configure the binary. In-loop handler fields
/// (`hooks`, `permission_policy`) carry `Arc`-wrapped handlers consulted by
/// the runtime's reader.
#[derive(Clone)]
pub struct Options {
    /// System-prompt configuration forwarded to the runtime.
    pub system_prompt: SystemPromptConfig,
    /// Model override.
    pub model: Option<ModelId>,
    /// Per-request output-token ceiling forwarded to the Messages API.
    pub max_tokens: MaxTokens,
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
    /// Registered in-loop hooks.
    pub hooks: HookRegistry,
    /// Optional tool-permission policy.
    pub permission_policy: Option<Arc<dyn PermissionPolicy>>,
    /// Optional model judge consulted under `PermissionMode::Auto`.
    pub permission_judge: Option<Arc<dyn PermissionJudge>>,
    /// Registered in-process MCP servers, held by the SDK.
    pub sdk_mcp_servers: SdkMcpRegistry,
    /// Schema-constrained structured output forwarded to the runtime.
    pub output_format: Option<OutputConfig>,
    /// Programmatic subagents the running agent can delegate to, keyed by the
    /// name the model invokes.
    pub agents: HashMap<String, AgentDefinition>,
    /// Session continuation intent for this session.
    pub session: SessionControl,
    /// Native session-store root (API runtime only; ignored by the CLI
    /// runtime). `None` selects the runtime's default store location.
    pub session_dir: Option<PathBuf>,
}

impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Options")
            .field("system_prompt", &self.system_prompt)
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .field("permission_mode", &self.permission_mode)
            .field("allowed_tools", &self.allowed_tools)
            .field("disallowed_tools", &self.disallowed_tools)
            .field("mcp_servers", &self.mcp_servers)
            .field("cwd", &self.cwd)
            .field("env", &self.env)
            .field("max_turns", &self.max_turns)
            .field("path_to_executable", &self.path_to_executable)
            .field("executable_args", &self.executable_args)
            .field("require_min_version", &self.require_min_version)
            .field("shutdown_grace", &self.shutdown_grace)
            .field(
                "hooks",
                &format_args!("<{} registered>", i32::from(!self.hooks.is_empty())),
            )
            .field("permission_policy", &self.permission_policy.is_some())
            .field("permission_judge", &self.permission_judge.is_some())
            .field(
                "sdk_mcp_servers",
                &format_args!(
                    "<{} registered>",
                    i32::from(!self.sdk_mcp_servers.is_empty())
                ),
            )
            .field("output_format", &self.output_format)
            .field(
                "agents",
                &format_args!("<{} registered>", self.agents.len()),
            )
            .field("session", &self.session)
            .field("session_dir", &self.session_dir)
            .finish()
    }
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
#[derive(Clone, Default)]
pub struct OptionsBuilder {
    system_prompt: SystemPromptConfig,
    model: Option<ModelId>,
    max_tokens: Option<MaxTokens>,
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
    hooks: HookRegistry,
    permission_policy: Option<Arc<dyn PermissionPolicy>>,
    permission_judge: Option<Arc<dyn PermissionJudge>>,
    sdk_mcp_servers: SdkMcpRegistry,
    output_format: Option<OutputConfig>,
    agents: HashMap<String, AgentDefinition>,
    session: SessionControl,
    session_dir: Option<PathBuf>,
}

impl fmt::Debug for OptionsBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OptionsBuilder")
            .field("system_prompt", &self.system_prompt)
            .field("model", &self.model)
            .field("permission_mode", &self.permission_mode)
            .field(
                "hooks",
                &format_args!("<{} registered>", i32::from(!self.hooks.is_empty())),
            )
            .field("permission_policy", &self.permission_policy.is_some())
            .finish_non_exhaustive()
    }
}

impl OptionsBuilder {
    /// Set the system prompt from a string or [`SystemPromptConfig`].
    #[must_use]
    pub fn system_prompt(mut self, prompt: impl Into<SystemPromptConfig>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Set the `claude_code` preset system prompt, optionally appended to.
    #[must_use]
    pub fn system_prompt_preset(
        mut self,
        append: Option<String>,
        exclude_dynamic_sections: bool,
    ) -> Self {
        self.system_prompt = SystemPromptConfig::Preset {
            append,
            exclude_dynamic_sections,
        };
        self
    }

    /// Set the model.
    #[must_use]
    pub fn model(mut self, model: ModelId) -> Self {
        self.model = Some(model);
        self
    }

    /// Set the per-request output-token ceiling.
    #[must_use]
    pub const fn max_tokens(mut self, max_tokens: MaxTokens) -> Self {
        self.max_tokens = Some(max_tokens);
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

    /// Register a hook for `event`, optionally narrowed by a `matcher`.
    #[must_use]
    pub fn hook(mut self, event: HookEvent, matcher: Option<String>, hook: Arc<dyn Hook>) -> Self {
        self.hooks.register(event, matcher, hook);
        self
    }

    /// Set the tool-permission policy.
    #[must_use]
    pub fn permission_policy(mut self, policy: Arc<dyn PermissionPolicy>) -> Self {
        self.permission_policy = Some(policy);
        self
    }

    /// Set the model judge consulted under `PermissionMode::Auto`.
    #[must_use]
    pub fn permission_judge(mut self, judge: Arc<dyn PermissionJudge>) -> Self {
        self.permission_judge = Some(judge);
        self
    }

    /// Register an in-process MCP server for this session.
    #[must_use]
    pub fn sdk_mcp_server(mut self, server: SdkMcpServer) -> Self {
        self.sdk_mcp_servers.register(server);
        self
    }

    /// Constrain the result to a JSON Schema via an [`OutputConfig`].
    #[must_use]
    pub fn output_format(mut self, config: impl Into<OutputConfig>) -> Self {
        self.output_format = Some(config.into());
        self
    }

    /// Constrain the result to the given JSON Schema.
    ///
    /// Convenience over [`OutputConfig::json_schema`] for the common case of a
    /// single JSON Schema value.
    #[must_use]
    pub fn output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_format = Some(OutputConfig::json_schema(schema));
        self
    }

    /// Register a programmatic subagent under `name`. A later registration
    /// with the same name replaces an earlier one.
    #[must_use]
    pub fn agent(mut self, name: impl Into<String>, definition: AgentDefinition) -> Self {
        self.agents.insert(name.into(), definition);
        self
    }

    /// Set the session continuation intent.
    #[must_use]
    pub fn session(mut self, session: SessionControl) -> Self {
        self.session = session;
        self
    }

    /// Set the native session-store root (API runtime only).
    #[must_use]
    pub fn session_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.session_dir = Some(dir.into());
        self
    }

    /// Finalize into an [`Options`].
    ///
    /// # Panics
    ///
    /// Never panics in practice: the fallback `max_tokens` default is built
    /// from a non-zero constant, so [`MaxTokens::new`] cannot fail there.
    #[must_use]
    pub fn build(self) -> Options {
        Options {
            system_prompt: self.system_prompt,
            model: self.model,
            max_tokens: self.max_tokens.unwrap_or_else(|| {
                #[expect(
                    clippy::expect_used,
                    reason = "DEFAULT_MAX_TOKENS is a non-zero constant; construction is infallible"
                )]
                MaxTokens::new(DEFAULT_MAX_TOKENS).expect("DEFAULT_MAX_TOKENS is non-zero")
            }),
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
            hooks: self.hooks,
            permission_policy: self.permission_policy,
            permission_judge: self.permission_judge,
            sdk_mcp_servers: self.sdk_mcp_servers,
            output_format: self.output_format,
            agents: self.agents,
            session: self.session,
            session_dir: self.session_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::sync::Arc;
    use std::time::Duration;

    use super::Options;
    use crate::agent::capabilities::HookEvent;
    use crate::agent::hooks::{Hook, HookInput, HookOutput};
    use crate::agent::permissions::{
        PermissionContext, PermissionDecision, PermissionMode, PermissionPolicy,
    };
    use crate::types::ModelId;

    struct TestHook;

    #[async_trait::async_trait]
    impl Hook for TestHook {
        async fn call(
            &self,
            _input: HookInput,
        ) -> Result<HookOutput, crate::agent::error::AgentError> {
            Ok(HookOutput::default())
        }
    }

    struct TestPolicy;

    #[async_trait::async_trait]
    impl PermissionPolicy for TestPolicy {
        async fn can_use_tool(
            &self,
            _tool: &str,
            _input: &serde_json::Value,
            _ctx: PermissionContext,
        ) -> Result<PermissionDecision, crate::agent::error::AgentError> {
            Ok(PermissionDecision::allow())
        }
    }

    struct TestJudge;

    #[async_trait::async_trait]
    impl crate::agent::permissions::PermissionJudge for TestJudge {
        async fn judge(
            &self,
            _req: &crate::agent::permissions::JudgeRequest<'_>,
        ) -> Result<PermissionDecision, crate::agent::error::AgentError> {
            Ok(PermissionDecision::allow())
        }
    }

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
    fn default_max_tokens_is_the_documented_constant() {
        let opts = Options::builder().build();
        assert_eq!(opts.max_tokens.get(), 4096);
    }

    #[test]
    fn builder_overrides_max_tokens() {
        let opts = Options::builder()
            .max_tokens(crate::types::MaxTokens::new(512).expect("non-zero"))
            .build();
        assert_eq!(opts.max_tokens.get(), 512);
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
        assert_eq!(
            opts.model.as_ref().map(ModelId::as_str),
            Some("claude-sonnet-4-5")
        );
        assert_eq!(opts.permission_mode, PermissionMode::AcceptEdits);
        assert_eq!(opts.allowed_tools, vec!["Bash".to_string()]);
        assert_eq!(opts.max_turns, Some(7));
        assert_eq!(opts.shutdown_grace, Duration::from_secs(2));
    }

    #[test]
    fn builder_accumulates_hooks_and_policy() {
        let opts = Options::builder()
            .hook(
                HookEvent::PreToolUse,
                Some("Bash".to_string()),
                Arc::new(TestHook),
            )
            .permission_policy(Arc::new(TestPolicy))
            .build();
        assert!(!opts.hooks.is_empty());
        assert!(opts.permission_policy.is_some());
    }

    #[test]
    fn debug_does_not_expose_handler_internals() {
        let opts = Options::builder()
            .permission_policy(Arc::new(TestPolicy))
            .build();
        let shown = format!("{opts:?}");
        assert!(shown.contains("permission_policy"));
    }

    #[test]
    fn default_options_have_no_handlers() {
        let opts = Options::default();
        assert!(opts.hooks.is_empty());
        assert!(opts.permission_policy.is_none());
    }

    #[test]
    fn builder_accumulates_sdk_mcp_servers() {
        use crate::agent::mcp::SdkMcpServer;
        let opts = Options::builder()
            .sdk_mcp_server(SdkMcpServer::builder("calc").build())
            .build();
        assert!(!opts.sdk_mcp_servers.is_empty());
    }

    #[test]
    fn default_options_have_no_sdk_mcp_servers() {
        assert!(Options::default().sdk_mcp_servers.is_empty());
    }

    #[test]
    fn builder_sets_permission_judge() {
        let opts = Options::builder()
            .permission_judge(Arc::new(TestJudge))
            .build();
        assert!(opts.permission_judge.is_some());
    }

    #[test]
    fn default_options_have_no_judge() {
        assert!(Options::default().permission_judge.is_none());
    }

    #[test]
    fn default_system_prompt_is_none() {
        use crate::agent::system_prompt::SystemPromptConfig;
        assert_eq!(Options::default().system_prompt, SystemPromptConfig::None);
    }

    #[test]
    fn builder_string_sets_text_variant() {
        use crate::agent::system_prompt::SystemPromptConfig;
        let opts = Options::builder().system_prompt("be brief").build();
        assert_eq!(
            opts.system_prompt,
            SystemPromptConfig::Text("be brief".to_owned())
        );
    }

    #[test]
    fn builder_preset_sets_preset_variant() {
        use crate::agent::system_prompt::SystemPromptConfig;
        let opts = Options::builder()
            .system_prompt_preset(Some("project rules".to_owned()), true)
            .build();
        assert_eq!(
            opts.system_prompt,
            SystemPromptConfig::Preset {
                append: Some("project rules".to_owned()),
                exclude_dynamic_sections: true,
            }
        );
    }

    #[test]
    fn output_schema_builder_sets_json_schema_config() {
        let opts = Options::builder()
            .output_schema(serde_json::json!({ "type": "object" }))
            .build();
        let cfg = opts.output_format.expect("output_format set");
        let j = serde_json::to_value(&cfg).expect("serialize");
        assert_eq!(j["format"]["type"], "json_schema");
    }

    #[test]
    fn output_format_defaults_to_none() {
        assert!(Options::default().output_format.is_none());
    }

    #[test]
    fn default_session_is_new_and_dir_is_none() {
        use crate::agent::types::SessionControl;
        let opts = Options::default();
        assert_eq!(opts.session, SessionControl::New);
        assert!(opts.session_dir.is_none());
    }

    #[test]
    fn builder_sets_session_and_dir() {
        use crate::agent::types::{SessionControl, SessionId};
        let opts = Options::builder()
            .session(SessionControl::Resume {
                id: SessionId::new("sess_x"),
                fork: true,
            })
            .session_dir("/tmp/clauders-sessions")
            .build();
        assert_eq!(
            opts.session,
            SessionControl::Resume {
                id: SessionId::new("sess_x"),
                fork: true,
            }
        );
        assert_eq!(
            opts.session_dir.as_deref(),
            Some(std::path::Path::new("/tmp/clauders-sessions"))
        );
    }

    #[test]
    fn agents_default_empty_and_builder_registers_by_name() {
        use crate::agent::subagents::AgentDefinition;

        let opts = Options::builder().build();
        assert!(opts.agents.is_empty());

        let reviewer = AgentDefinition::new("reviewer", "be careful").expect("valid");
        let opts = Options::builder().agent("reviewer", reviewer).build();
        assert_eq!(opts.agents.len(), 1);
        assert_eq!(
            opts.agents.get("reviewer").expect("present").prompt(),
            "be careful"
        );
    }
}
