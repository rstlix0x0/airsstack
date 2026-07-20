//! Session configuration for the Agent SDK.

use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::agent::capabilities::HookEvent;
use crate::agent::elicitation::ElicitationPolicy;
use crate::agent::hooks::{Hook, HookRegistry};
use crate::agent::mcp::{SdkMcpRegistry, SdkMcpServer};
use crate::agent::permissions::{PermissionMode, PermissionPolicy};
use crate::agent::subagents::AgentDefinition;
use crate::agent::system_prompt::SystemPromptConfig;
use crate::agent::types::{
    BudgetUsd, EffortLevel, McpServerConfig, SessionControl, SessionId, SessionPersistence,
    SettingsSource,
};
use crate::messages::structured_outputs::OutputConfig;
use crate::types::{MaxTokens, ModelId};

/// Default graceful-shutdown window before the supervisor forces a kill.
const DEFAULT_SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// Default bound on how long a control request waits for its correlated
/// response, mirroring the official Python Agent SDK's default.
const DEFAULT_CONTROL_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Default per-request output-token ceiling when the caller sets none.
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Per-chunk stderr callback: invoked with one valid UTF-8 chunk at a time.
type StderrCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Configuration for a `Client` / `query` session.
///
/// Built via [`Options::builder`]. Carries everything the runtime needs to
/// discover, spawn, and configure the binary. In-loop handler fields
/// (`hooks`, `permission_policy`) carry `Arc`-wrapped handlers consulted by
/// the runtime's reader.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent CLI toggle flag mirrored 1:1 from the binary's own flag surface, not a combined state machine"
)]
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
    /// Bound on how long a control request (e.g. `interrupt`, `set_model`)
    /// waits for its correlated response before failing with
    /// [`AgentError::ControlRequestTimedOut`](crate::agent::error::AgentError::ControlRequestTimedOut).
    pub control_request_timeout: Duration,
    /// Registered in-loop hooks.
    pub hooks: HookRegistry,
    /// Optional tool-permission policy.
    pub permission_policy: Option<Arc<dyn PermissionPolicy>>,
    /// Optional MCP elicitation policy.
    pub elicitation_policy: Option<Arc<dyn ElicitationPolicy>>,
    /// Registered in-process MCP servers, held by the SDK.
    pub sdk_mcp_servers: SdkMcpRegistry,
    /// Schema-constrained structured output forwarded to the runtime.
    pub output_format: Option<OutputConfig>,
    /// Programmatic subagents the running agent can delegate to, keyed by the
    /// name the model invokes.
    pub agents: HashMap<String, AgentDefinition>,
    /// Session continuation intent for this session.
    pub session: SessionControl,
    /// Force a specific session id (official `sessionId`).
    ///
    /// Lowers to `--session-id <id>` for a new session; the binary requires
    /// a valid UUID and rejects any other value.
    pub session_id: Option<SessionId>,
    /// Display title for the session, sent in the initialize handshake.
    pub title: Option<String>,
    /// Whether the binary persists this session.
    pub session_persistence: SessionPersistence,
    /// Model to fall back to if the primary model is overloaded.
    pub fallback_model: Option<ModelId>,
    /// Use only `--mcp-config` servers; ignore project/user/plugin MCP config.
    pub strict_mcp_config: bool,
    /// Extra directories the binary's tools may access.
    pub add_dirs: Vec<PathBuf>,
    /// Session settings source (file path or inline JSON).
    pub settings: Option<SettingsSource>,
    /// Client-side spend ceiling for the session.
    pub max_budget_usd: Option<BudgetUsd>,
    /// Emit partial-message stream frames as the turn streams.
    pub include_partial_messages: bool,
    /// Name of the MCP tool that answers permission prompts, overriding the
    /// SDK's default `stdio` bridge when set.
    pub permission_prompt_tool_name: Option<String>,
    /// Opaque caller identifier. Reserved for API-shape parity; has no effect
    /// on the CLI runtime (the binary exposes no matching flag).
    pub user: Option<String>,
    /// Emit hook-lifecycle observability frames on the message stream.
    pub include_hook_events: bool,
    /// Per-chunk callback for the child's stderr (augments capture; does not
    /// suppress it).
    pub stderr: Option<StderrCallback>,
    /// Cap on bytes buffered per stdout line before erroring (`None` =
    /// unbounded).
    pub max_buffer_size: Option<NonZeroUsize>,
    /// Reasoning-effort level for the session (→ `--effort <level>`).
    pub effort: Option<EffortLevel>,
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
            .field("control_request_timeout", &self.control_request_timeout)
            .field(
                "hooks",
                &format_args!("<{} registered>", i32::from(!self.hooks.is_empty())),
            )
            .field("permission_policy", &self.permission_policy.is_some())
            .field("elicitation_policy", &self.elicitation_policy.is_some())
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
            .field("session_id", &self.session_id)
            .field("title", &self.title)
            .field("session_persistence", &self.session_persistence)
            .field("fallback_model", &self.fallback_model)
            .field("strict_mcp_config", &self.strict_mcp_config)
            .field("add_dirs", &self.add_dirs)
            .field("settings", &self.settings)
            .field("max_budget_usd", &self.max_budget_usd)
            .field("include_partial_messages", &self.include_partial_messages)
            .field(
                "permission_prompt_tool_name",
                &self.permission_prompt_tool_name,
            )
            .field("user", &self.user)
            .field("include_hook_events", &self.include_hook_events)
            .field("stderr", &self.stderr.is_some())
            .field("max_buffer_size", &self.max_buffer_size)
            .field("effort", &self.effort)
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors Options: each bool is an independent CLI toggle flag, not a combined state machine"
)]
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
    control_request_timeout: Option<Duration>,
    hooks: HookRegistry,
    permission_policy: Option<Arc<dyn PermissionPolicy>>,
    elicitation_policy: Option<Arc<dyn ElicitationPolicy>>,
    sdk_mcp_servers: SdkMcpRegistry,
    output_format: Option<OutputConfig>,
    agents: HashMap<String, AgentDefinition>,
    session: SessionControl,
    session_id: Option<SessionId>,
    title: Option<String>,
    session_persistence: SessionPersistence,
    fallback_model: Option<ModelId>,
    strict_mcp_config: bool,
    add_dirs: Vec<PathBuf>,
    settings: Option<SettingsSource>,
    max_budget_usd: Option<BudgetUsd>,
    include_partial_messages: bool,
    permission_prompt_tool_name: Option<String>,
    user: Option<String>,
    include_hook_events: bool,
    stderr: Option<StderrCallback>,
    max_buffer_size: Option<NonZeroUsize>,
    effort: Option<EffortLevel>,
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

    /// Override the control-request response timeout.
    #[must_use]
    pub const fn control_request_timeout(mut self, timeout: Duration) -> Self {
        self.control_request_timeout = Some(timeout);
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

    /// Set the MCP elicitation policy.
    #[must_use]
    pub fn elicitation_policy(mut self, policy: Arc<dyn ElicitationPolicy>) -> Self {
        self.elicitation_policy = Some(policy);
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

    /// Force a specific session id for a new session.
    ///
    /// The binary requires a valid UUID and rejects any other value.
    #[must_use]
    pub fn session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    /// Set the session's display title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set whether the binary persists this session.
    #[must_use]
    pub const fn session_persistence(mut self, persistence: SessionPersistence) -> Self {
        self.session_persistence = persistence;
        self
    }

    /// Set the fallback model.
    #[must_use]
    pub fn fallback_model(mut self, model: ModelId) -> Self {
        self.fallback_model = Some(model);
        self
    }

    /// Restrict MCP servers to those from `--mcp-config` only.
    #[must_use]
    pub const fn strict_mcp_config(mut self, strict: bool) -> Self {
        self.strict_mcp_config = strict;
        self
    }

    /// Append a directory the binary's tools may access.
    #[must_use]
    pub fn add_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.add_dirs.push(dir.into());
        self
    }

    /// Set the session settings source to a settings JSON file path.
    #[must_use]
    pub fn settings_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.settings = Some(SettingsSource::Path(path.into()));
        self
    }

    /// Set the session settings source to inline settings JSON.
    #[must_use]
    pub fn settings_inline(mut self, value: serde_json::Value) -> Self {
        self.settings = Some(SettingsSource::Inline(value));
        self
    }

    /// Set the client-side spend ceiling.
    #[must_use]
    pub const fn max_budget_usd(mut self, budget: BudgetUsd) -> Self {
        self.max_budget_usd = Some(budget);
        self
    }

    /// Set the reasoning-effort level for the session.
    #[must_use]
    pub const fn effort(mut self, effort: EffortLevel) -> Self {
        self.effort = Some(effort);
        self
    }

    /// Enable partial-message stream frames.
    #[must_use]
    pub const fn include_partial_messages(mut self, include: bool) -> Self {
        self.include_partial_messages = include;
        self
    }

    /// Override the permission-prompt MCP tool name.
    #[must_use]
    pub fn permission_prompt_tool_name(mut self, name: impl Into<String>) -> Self {
        self.permission_prompt_tool_name = Some(name.into());
        self
    }

    /// Set the opaque caller identifier (reserved; no CLI effect).
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Emit hook-lifecycle observability frames on the message stream.
    #[must_use]
    pub const fn include_hook_events(mut self, include: bool) -> Self {
        self.include_hook_events = include;
        self
    }

    /// Set a per-chunk stderr callback (augments capture; does not suppress).
    #[must_use]
    pub fn stderr(mut self, callback: impl Fn(&str) + Send + Sync + 'static) -> Self {
        self.stderr = Some(Arc::new(callback));
        self
    }

    /// Cap bytes buffered per stdout line before erroring.
    #[must_use]
    pub const fn max_buffer_size(mut self, cap: NonZeroUsize) -> Self {
        self.max_buffer_size = Some(cap);
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
            control_request_timeout: self
                .control_request_timeout
                .unwrap_or(DEFAULT_CONTROL_REQUEST_TIMEOUT),
            hooks: self.hooks,
            permission_policy: self.permission_policy,
            elicitation_policy: self.elicitation_policy,
            sdk_mcp_servers: self.sdk_mcp_servers,
            output_format: self.output_format,
            agents: self.agents,
            session: self.session,
            session_id: self.session_id,
            title: self.title,
            session_persistence: self.session_persistence,
            fallback_model: self.fallback_model,
            strict_mcp_config: self.strict_mcp_config,
            add_dirs: self.add_dirs,
            settings: self.settings,
            max_budget_usd: self.max_budget_usd,
            include_partial_messages: self.include_partial_messages,
            permission_prompt_tool_name: self.permission_prompt_tool_name,
            user: self.user,
            include_hook_events: self.include_hook_events,
            stderr: self.stderr,
            max_buffer_size: self.max_buffer_size,
            effort: self.effort,
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

    #[test]
    fn defaults_are_sane() {
        let opts = Options::builder().build();
        assert_eq!(opts.permission_mode, PermissionMode::Default);
        assert_eq!(opts.shutdown_grace, Duration::from_secs(5));
        assert_eq!(opts.control_request_timeout, Duration::from_secs(60));
        assert!(!opts.require_min_version);
        assert!(opts.model.is_none());
        assert!(opts.allowed_tools.is_empty());
    }

    #[test]
    fn builder_overrides_control_request_timeout() {
        let opts = Options::builder()
            .control_request_timeout(Duration::from_secs(10))
            .build();
        assert_eq!(opts.control_request_timeout, Duration::from_secs(10));
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
    fn default_session_is_new() {
        use crate::agent::types::SessionControl;
        assert_eq!(Options::default().session, SessionControl::New);
    }

    #[test]
    fn builder_sets_session() {
        use crate::agent::types::{SessionControl, SessionId};
        let opts = Options::builder()
            .session(SessionControl::Resume {
                id: SessionId::new("sess_x"),
                fork: true,
            })
            .build();
        assert_eq!(
            opts.session,
            SessionControl::Resume {
                id: SessionId::new("sess_x"),
                fork: true,
            }
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

    #[test]
    fn new_flag_surface_defaults_are_empty() {
        let opts = Options::default();
        assert!(opts.fallback_model.is_none());
        assert!(!opts.strict_mcp_config);
        assert!(opts.add_dirs.is_empty());
        assert!(opts.settings.is_none());
        assert!(opts.max_budget_usd.is_none());
        assert!(!opts.include_partial_messages);
        assert!(opts.permission_prompt_tool_name.is_none());
        assert!(opts.user.is_none());
    }

    #[test]
    fn builder_sets_new_flag_surface() {
        use crate::agent::types::{BudgetUsd, SettingsSource};
        use crate::types::ModelId;

        let opts = Options::builder()
            .fallback_model(ModelId::custom("claude-haiku-4-5").expect("model"))
            .strict_mcp_config(true)
            .add_dir("/repo/a")
            .add_dir("/repo/b")
            .settings_path("/etc/s.json")
            .max_budget_usd(BudgetUsd::new(5.0).expect("positive"))
            .include_partial_messages(true)
            .permission_prompt_tool_name("mcp__gate__approve")
            .user("user-123")
            .build();

        assert_eq!(
            opts.fallback_model.as_ref().map(ModelId::as_str),
            Some("claude-haiku-4-5")
        );
        assert!(opts.strict_mcp_config);
        assert_eq!(
            opts.add_dirs,
            vec![
                std::path::PathBuf::from("/repo/a"),
                std::path::PathBuf::from("/repo/b")
            ]
        );
        assert_eq!(
            opts.settings,
            Some(SettingsSource::Path("/etc/s.json".into()))
        );
        assert_eq!(opts.max_budget_usd.map(BudgetUsd::get), Some(5.0));
        assert!(opts.include_partial_messages);
        assert_eq!(
            opts.permission_prompt_tool_name.as_deref(),
            Some("mcp__gate__approve")
        );
        assert_eq!(opts.user.as_deref(), Some("user-123"));
    }

    #[test]
    fn settings_inline_variant_is_accepted() {
        use crate::agent::types::SettingsSource;
        let opts = Options::builder()
            .settings_inline(serde_json::json!({ "k": 1 }))
            .build();
        assert!(matches!(opts.settings, Some(SettingsSource::Inline(_))));
    }

    #[test]
    fn runtime_behavior_knob_defaults() {
        let opts = Options::default();
        assert!(!opts.include_hook_events);
        assert!(opts.stderr.is_none());
        assert!(opts.max_buffer_size.is_none());
    }

    #[test]
    fn builder_sets_runtime_behavior_knobs() {
        use std::num::NonZeroUsize;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let hits = Arc::new(AtomicUsize::new(0));
        let hits2 = Arc::clone(&hits);
        let opts = Options::builder()
            .include_hook_events(true)
            .stderr(move |_line: &str| {
                hits2.fetch_add(1, Ordering::Relaxed);
            })
            .max_buffer_size(NonZeroUsize::new(4096).expect("nonzero"))
            .build();

        assert!(opts.include_hook_events);
        assert!(opts.stderr.is_some());
        assert_eq!(opts.max_buffer_size.map(NonZeroUsize::get), Some(4096));
        // The stored callback is invocable. `Arc<dyn Fn>` is not itself callable,
        // so deref to `&dyn Fn` (which does implement `Fn`) before calling.
        let cb = opts.stderr.as_deref().expect("set");
        cb("boom");
        assert_eq!(hits.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn debug_shows_stderr_as_bool_without_leaking_the_closure() {
        let opts = Options::builder().stderr(|_l: &str| {}).build();
        let shown = format!("{opts:?}");
        assert!(shown.contains("stderr: true"), "got: {shown}");
    }

    #[test]
    fn effort_defaults_to_none_and_builder_sets_it() {
        use crate::agent::types::EffortLevel;

        assert!(Options::default().effort.is_none());

        let opts = Options::builder().effort(EffortLevel::High).build();
        assert_eq!(opts.effort, Some(EffortLevel::High));
    }

    #[test]
    fn session_config_knobs_default_and_round_trip() {
        use crate::agent::types::{SessionId, SessionPersistence};

        let defaults = Options::default();
        assert!(defaults.session_id.is_none());
        assert!(defaults.title.is_none());
        assert_eq!(defaults.session_persistence, SessionPersistence::Enabled);

        let opts = Options::builder()
            .session_id(SessionId::new("sess_7"))
            .title("Nightly triage")
            .session_persistence(SessionPersistence::Disabled)
            .build();
        assert_eq!(
            opts.session_id.as_ref().map(SessionId::as_str),
            Some("sess_7")
        );
        assert_eq!(opts.title.as_deref(), Some("Nightly triage"));
        assert_eq!(opts.session_persistence, SessionPersistence::Disabled);
    }

    #[test]
    fn elicitation_policy_is_registered_and_defaults_to_none() {
        use crate::agent::elicitation::{
            ElicitationPolicy, ElicitationRequest, ElicitationResponse,
        };
        use crate::agent::error::AgentError;
        use std::sync::Arc;

        struct H;
        #[async_trait::async_trait]
        impl ElicitationPolicy for H {
            async fn elicit(
                &self,
                _r: ElicitationRequest,
            ) -> Result<ElicitationResponse, AgentError> {
                Ok(ElicitationResponse::Decline)
            }
        }

        assert!(Options::default().elicitation_policy.is_none());
        let opts = Options::builder().elicitation_policy(Arc::new(H)).build();
        assert!(opts.elicitation_policy.is_some());
    }
}
