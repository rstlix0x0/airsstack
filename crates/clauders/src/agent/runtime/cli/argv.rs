//! Mapping session options to the backend's argument vector.

use crate::agent::error::AgentError;
use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;
use crate::agent::system_prompt::SystemPromptConfig;
use crate::agent::types::{
    SandboxConfig, SessionControl, SessionPersistence, SettingsSource, Skills, ThinkingMode,
};
use crate::messages::structured_outputs::OutputFormat;

/// The allowed-tools list with skill entries folded in, deduped, mirroring the
/// official SDK: [`Skills::All`] appends `Skill`; each named skill appends
/// `Skill(<name>)`, skipping any already present in `allowed_tools`.
fn effective_allowed_tools(options: &Options) -> Vec<String> {
    let mut tools = options.allowed_tools.clone();
    if let Some(skills) = &options.skills {
        let additions: Vec<String> = match skills {
            Skills::All => vec!["Skill".to_string()],
            Skills::List(names) => names.iter().map(|n| format!("Skill({n})")).collect(),
        };
        for addition in additions {
            if !tools.contains(&addition) {
                tools.push(addition);
            }
        }
    }
    tools
}

/// Return the sandbox config with `fail_if_unavailable` defaulted to `true`
/// when `enabled == Some(true)` and it is unset, matching the official SDK.
fn defaulted_sandbox(sandbox: &SandboxConfig) -> SandboxConfig {
    let mut out = sandbox.clone();
    if out.enabled == Some(true) && out.fail_if_unavailable.is_none() {
        out.fail_if_unavailable = Some(true);
    }
    out
}

/// Resolve the single `--settings` argument value from the settings source and
/// the sandbox config, mirroring the official SDK's merge:
/// - neither set → `Ok(None)` (no `--settings`);
/// - sandbox with a settings *file path* → `Err(SandboxWithSettingsPath)`;
/// - sandbox alone → `{"sandbox": …}`;
/// - sandbox with inline settings → the inline object with `sandbox` inserted;
/// - settings alone → its existing lowering, unchanged.
fn effective_settings_arg(
    settings: Option<&SettingsSource>,
    sandbox: Option<&SandboxConfig>,
) -> Result<Option<String>, AgentError> {
    match (settings, sandbox) {
        (Some(SettingsSource::Path(path)), None) => Ok(Some(path.to_string_lossy().into_owned())),
        (Some(SettingsSource::Inline(value)), None) => Ok(Some(value.to_string())),
        (None, None) => Ok(None),
        (Some(SettingsSource::Path(_)), Some(_)) => Err(AgentError::SandboxWithSettingsPath),
        (None, Some(sb)) => {
            let merged = serde_json::json!({ "sandbox": defaulted_sandbox(sb) });
            Ok(Some(merged.to_string()))
        }
        (Some(SettingsSource::Inline(value)), Some(sb)) => {
            let mut obj = value.as_object().cloned().unwrap_or_default();
            obj.insert(
                "sandbox".to_string(),
                serde_json::to_value(defaulted_sandbox(sb)).unwrap_or(serde_json::Value::Null),
            );
            Ok(Some(serde_json::Value::Object(obj).to_string()))
        }
    }
}

/// Build the full argument vector for spawning the backend.
///
/// Caller-supplied `executable_args` come first, then the SDK-managed
/// stream-protocol flags, then mapped option fields. `cwd` and `env` are not
/// argv — they are applied to the process spawn config.
pub(super) fn build_argv(options: &Options) -> Result<Vec<String>, AgentError> {
    let mut argv: Vec<String> = options.executable_args.clone();

    argv.push("--output-format".to_string());
    argv.push("stream-json".to_string());
    argv.push("--input-format".to_string());
    argv.push("stream-json".to_string());
    argv.push("--verbose".to_string());

    argv.push("--permission-mode".to_string());
    argv.push(permission_mode_wire(options.permission_mode).to_string());

    // A caller-named prompt tool takes precedence; otherwise a registered
    // policy routes prompts over the control protocol via the `stdio` sentinel.
    if let Some(name) = &options.permission_prompt_tool_name {
        argv.push("--permission-prompt-tool".to_string());
        argv.push(name.clone());
    } else if options.permission_policy.is_some() {
        argv.push("--permission-prompt-tool".to_string());
        argv.push("stdio".to_string());
    }

    if let Some(model) = &options.model {
        argv.push("--model".to_string());
        argv.push(model.as_str().to_string());
    }
    match &options.system_prompt {
        SystemPromptConfig::None => {}
        SystemPromptConfig::Text(text) => {
            argv.push("--system-prompt".to_string());
            argv.push(text.clone());
        }
        SystemPromptConfig::Preset {
            append,
            exclude_dynamic_sections,
        } => {
            // The CLI's built-in base prompt *is* the claude_code preset, so keep
            // it and append rather than replacing it via --system-prompt.
            if let Some(append) = append {
                argv.push("--append-system-prompt".to_string());
                argv.push(append.clone());
            }
            if *exclude_dynamic_sections {
                argv.push("--exclude-dynamic-system-prompt-sections".to_string());
            }
        }
    }
    let allowed = effective_allowed_tools(options);
    if !allowed.is_empty() {
        argv.push("--allowed-tools".to_string());
        argv.push(allowed.join(","));
    }
    if !options.disallowed_tools.is_empty() {
        argv.push("--disallowed-tools".to_string());
        argv.push(options.disallowed_tools.join(","));
    }
    if let Some(max_turns) = options.max_turns {
        argv.push("--max-turns".to_string());
        argv.push(max_turns.to_string());
    }
    // Structured output. The binary reads the schema from `--json-schema`, and
    // wants the bare schema object; `OutputConfig` wraps it as
    // `{"format":{"type":"json_schema","schema":…}}`, so unwrap before emitting.
    // The initialize handshake also carries `output_format` (see
    // `handshake.rs`), but the binary's bridge acks that request without
    // reading the field — the flag is what actually reaches the model.
    //
    // `OutputConfig::effort` is deliberately not lowered here: the session's
    // reasoning effort is `Options::effort`, which maps to `--effort`.
    if let Some(OutputFormat::JsonSchema { schema }) = options
        .output_format
        .as_ref()
        .and_then(|config| config.format.as_ref())
    {
        argv.push("--json-schema".to_string());
        argv.push(schema.to_string());
    }
    // Each `--mcp-config` payload must be wrapped in `mcpServers`: the binary
    // validates it against a schema whose only key is that one, and rejects an
    // unwrapped server map at startup before the session begins.
    for server in &options.mcp_servers {
        argv.push("--mcp-config".to_string());
        let config = serde_json::json!({ "mcpServers": { server.name(): server.config() } });
        argv.push(config.to_string());
    }
    for declaration in options.sdk_mcp_servers.declarations() {
        argv.push("--mcp-config".to_string());
        argv.push(declaration.to_string());
    }
    // Programmatic subagents: forward the whole map as one JSON object, mirroring
    // the `--mcp-config` pattern. The binary owns subagent execution; programmatic
    // definitions take precedence over any filesystem `.claude/agents/*.md` of the
    // same name. Flag acceptance is verified behind CLAUDERS_AGENT_E2E=1.
    if !options.agents.is_empty() {
        argv.push("--agents".to_string());
        let config = serde_json::json!(options.agents);
        argv.push(config.to_string());
    }
    if let Some(model) = &options.fallback_model {
        argv.push("--fallback-model".to_string());
        argv.push(model.as_str().to_string());
    }
    if options.strict_mcp_config {
        argv.push("--strict-mcp-config".to_string());
    }
    if !options.add_dirs.is_empty() {
        // Variadic form: one flag, each directory as a following value.
        argv.push("--add-dir".to_string());
        for dir in &options.add_dirs {
            argv.push(dir.to_string_lossy().into_owned());
        }
    }
    if let Some(value) =
        effective_settings_arg(options.settings.as_ref(), options.sandbox.as_ref())?
    {
        argv.push("--settings".to_string());
        argv.push(value);
    }
    argv.extend(trailing_flag_args(options));
    argv.extend(session_args(options));
    Ok(argv)
}

/// Map the remaining scalar/presence flags — spend ceiling, partial-message
/// and hook-event streaming, reasoning effort, settings sources, beta flags,
/// local plugin directories, extended-thinking configuration, and the
/// singular agent selector — to argv.
fn trailing_flag_args(options: &Options) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(budget) = options.max_budget_usd {
        args.push("--max-budget-usd".to_string());
        args.push(budget.get().to_string());
    }
    if options.include_partial_messages {
        args.push("--include-partial-messages".to_string());
    }
    if options.include_hook_events {
        args.push("--include-hook-events".to_string());
    }
    if let Some(effort) = options.effort {
        args.push("--effort".to_string());
        args.push(effort.as_str().to_string());
    }
    if !options.setting_sources.is_empty() {
        let csv = options
            .setting_sources
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        args.push(format!("--setting-sources={csv}"));
    }
    if !options.betas.is_empty() {
        args.push("--betas".to_string());
        args.push(options.betas.join(","));
    }
    for plugin in &options.plugins {
        args.push(
            if plugin.skip_mcp_discovery {
                "--plugin-dir-no-mcp"
            } else {
                "--plugin-dir"
            }
            .to_string(),
        );
        args.push(plugin.path.to_string_lossy().into_owned());
    }
    // The thinking object wins; the scalar `max_thinking_tokens` is the fallback
    // (0 => disabled, n>0 => --max-thinking-tokens n), mirroring the SDK.
    if let Some(thinking) = &options.thinking {
        match thinking.mode {
            ThinkingMode::Enabled {
                budget_tokens: Some(n),
            } => {
                args.push("--max-thinking-tokens".to_string());
                args.push(n.to_string());
            }
            ThinkingMode::Enabled {
                budget_tokens: None,
            }
            | ThinkingMode::Adaptive => {
                args.push("--thinking".to_string());
                args.push("adaptive".to_string());
            }
            ThinkingMode::Disabled => {
                args.push("--thinking".to_string());
                args.push("disabled".to_string());
            }
        }
        if !matches!(thinking.mode, ThinkingMode::Disabled) {
            if let Some(display) = thinking.display {
                args.push("--thinking-display".to_string());
                args.push(display.as_str().to_string());
            }
        }
    } else if let Some(budget) = options.max_thinking_tokens {
        if budget == 0 {
            args.push("--thinking".to_string());
            args.push("disabled".to_string());
        } else {
            args.push("--max-thinking-tokens".to_string());
            args.push(budget.to_string());
        }
    }
    if let Some(agent) = &options.agent {
        args.push("--agent".to_string());
        args.push(agent.clone());
    }
    args
}

/// The backend's camelCase wire spelling for a permission mode.
pub(super) const fn permission_mode_wire(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Default => "default",
        PermissionMode::AcceptEdits => "acceptEdits",
        PermissionMode::Plan => "plan",
        PermissionMode::BypassPermissions => "bypassPermissions",
        PermissionMode::DontAsk => "dontAsk",
        PermissionMode::Auto => "auto",
    }
}

/// Map session identity, continuation, and persistence options to argv.
///
/// `--session-id` forces the id of a NEW session; it contradicts
/// `--continue`/`--resume`, which select an existing one, so it is emitted
/// only for a new session (with a warning logged on the contradictory
/// combination). `--fork-session` combines with either `--continue` or
/// `--resume <id>`; a new session with no forced id emits neither.
fn session_args(options: &Options) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(id) = &options.session_id {
        if matches!(options.session, SessionControl::New) {
            args.push("--session-id".to_string());
            args.push(id.as_str().to_string());
        } else {
            tracing::warn!("session_id is ignored when continuing or resuming a session");
        }
    }
    if options.session_persistence == SessionPersistence::Disabled {
        args.push("--no-session-persistence".to_string());
    }
    match &options.session {
        SessionControl::New => {}
        SessionControl::Continue { fork } => {
            args.push("--continue".to_string());
            if *fork {
                args.push("--fork-session".to_string());
            }
        }
        SessionControl::Resume {
            id,
            fork,
            resume_at,
        } => {
            args.push("--resume".to_string());
            args.push(id.as_str().to_string());
            if *fork {
                args.push("--fork-session".to_string());
            }
            if let Some(at) = resume_at {
                args.push(format!("--resume-session-at={}", at.as_str()));
            }
        }
    }
    args
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{build_argv, permission_mode_wire};
    use crate::agent::options::Options;
    use crate::agent::permissions::PermissionMode;
    use crate::agent::types::McpServerConfig;
    use crate::types::ModelId;

    #[test]
    fn always_emits_the_streaming_protocol_flags() {
        let argv = build_argv(&Options::default()).expect("build argv");
        let joined = argv.join(" ");
        assert!(joined.contains("--output-format stream-json"));
        assert!(joined.contains("--input-format stream-json"));
        assert!(joined.contains("--verbose"));
        assert!(joined.contains("--permission-mode default"));
    }

    #[test]
    fn maps_optional_fields_and_prepends_executable_args() {
        let opts = Options::builder()
            .executable_args(vec!["--mcp-debug".to_string()])
            .model(ModelId::custom("claude-sonnet-4-5").expect("model"))
            .system_prompt("be brief")
            .permission_mode(PermissionMode::AcceptEdits)
            .allowed_tools(vec!["Bash".into(), "Read".into()])
            .disallowed_tools(vec!["Write".into()])
            .max_turns(5)
            .mcp_servers(vec![McpServerConfig::new(
                "fs",
                serde_json::json!({"command": "node"}),
            )])
            .build();
        let argv = build_argv(&opts).expect("build argv");
        assert_eq!(argv.first().map(String::as_str), Some("--mcp-debug"));
        let joined = argv.join(" ");
        assert!(joined.contains("--model claude-sonnet-4-5"));
        assert!(joined.contains("--system-prompt be brief"));
        assert!(joined.contains("--permission-mode acceptEdits"));
        assert!(joined.contains("--allowed-tools Bash,Read"));
        assert!(joined.contains("--disallowed-tools Write"));
        assert!(joined.contains("--max-turns 5"));
        assert!(joined.contains("--mcp-config"));
        assert_eq!(
            mcp_config_payload(&argv),
            serde_json::json!({ "mcpServers": { "fs": { "command": "node" } } }),
            "an external server's payload must be wrapped in `mcpServers`"
        );
    }

    /// The value following the first `--mcp-config`, parsed back to JSON.
    ///
    /// Comparing the parsed value rather than a substring is deliberate: a
    /// `contains("\"fs\"")` check passes whether or not the server map is
    /// wrapped, which is exactly how an unwrapped payload once reached the
    /// binary and killed the session at startup.
    fn mcp_config_payload(argv: &[String]) -> serde_json::Value {
        let index = argv
            .iter()
            .position(|arg| arg == "--mcp-config")
            .expect("argv carries --mcp-config");
        let raw = argv.get(index + 1).expect("--mcp-config carries a value");
        serde_json::from_str(raw).expect("the payload is valid JSON")
    }

    #[test]
    fn output_schema_lowers_to_json_schema_flag_unwrapped() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "language": { "type": "string" } },
            "required": ["language"]
        });
        let opts = Options::builder().output_schema(schema.clone()).build();
        let argv = build_argv(&opts).expect("build argv");

        let index = argv
            .iter()
            .position(|arg| arg == "--json-schema")
            .expect("argv carries --json-schema");
        let raw = argv.get(index + 1).expect("--json-schema carries a value");
        let payload: serde_json::Value = serde_json::from_str(raw).expect("valid JSON");

        // The flag wants the bare schema. Emitting the `OutputConfig` wrapper
        // (`{"format":{"type":"json_schema","schema":…}}`) would hand the
        // binary a schema whose only property is `format`.
        assert_eq!(payload, schema);
        assert!(
            payload.get("format").is_none(),
            "the OutputConfig wrapper must not reach the flag: {payload}"
        );
    }

    #[test]
    fn omits_json_schema_flag_when_no_output_format_is_set() {
        let argv = build_argv(&Options::default()).expect("build argv");
        assert!(!argv.iter().any(|arg| arg == "--json-schema"));
    }

    #[test]
    fn permission_mode_wire_strings() {
        assert_eq!(permission_mode_wire(PermissionMode::Default), "default");
        assert_eq!(
            permission_mode_wire(PermissionMode::AcceptEdits),
            "acceptEdits"
        );
        assert_eq!(permission_mode_wire(PermissionMode::Plan), "plan");
        assert_eq!(
            permission_mode_wire(PermissionMode::BypassPermissions),
            "bypassPermissions"
        );
        assert_eq!(permission_mode_wire(PermissionMode::DontAsk), "dontAsk");
    }

    #[test]
    fn auto_mode_lowers_to_auto_wire() {
        assert_eq!(permission_mode_wire(PermissionMode::Auto), "auto");
    }

    #[test]
    fn omits_permission_prompt_tool_without_policy() {
        let argv = build_argv(&Options::default()).expect("build argv");
        assert!(!argv.iter().any(|a| a == "--permission-prompt-tool"));
    }

    #[test]
    fn emits_mcp_config_type_sdk_for_in_process_server() {
        use crate::agent::mcp::SdkMcpServer;
        let opts = Options::builder()
            .sdk_mcp_server(SdkMcpServer::builder("calc").build())
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let joined = argv.join(" ");
        assert!(joined.contains("--mcp-config"), "got: {joined}");
        // The declaration names the server, marks it `sdk`, and wraps the map
        // in `mcpServers` — the binary rejects the payload without the wrapper.
        assert_eq!(
            mcp_config_payload(&argv),
            serde_json::json!({ "mcpServers": { "calc": { "type": "sdk" } } }),
            "got: {joined}"
        );
    }

    #[test]
    fn emits_permission_prompt_tool_when_policy_set() {
        use crate::agent::error::AgentError;
        use crate::agent::permissions::{PermissionContext, PermissionDecision, PermissionPolicy};
        use std::sync::Arc;

        struct P;
        #[async_trait::async_trait]
        impl PermissionPolicy for P {
            async fn can_use_tool(
                &self,
                _t: &str,
                _i: &serde_json::Value,
                _c: PermissionContext,
                _cancel: crate::agent::cancel::CancelSignal,
            ) -> Result<PermissionDecision, AgentError> {
                Ok(PermissionDecision::allow())
            }
        }

        let opts = Options::builder().permission_policy(Arc::new(P)).build();
        let argv = build_argv(&opts).expect("build argv");
        let joined = argv.join(" ");
        assert!(
            joined.contains("--permission-prompt-tool stdio"),
            "got: {joined}"
        );
    }

    #[test]
    fn preset_maps_to_append_system_prompt() {
        let opts = Options::builder()
            .system_prompt_preset(Some("extra rules".to_owned()), true)
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let joined = argv.join(" ");
        assert!(
            joined.contains("--append-system-prompt extra rules"),
            "got: {joined}"
        );
        assert!(
            joined.contains("--exclude-dynamic-system-prompt-sections"),
            "got: {joined}"
        );
        // Preset keeps the CLI's built-in base; it must NOT replace via --system-prompt.
        assert!(!joined.contains("--system-prompt "), "got: {joined}");
    }

    #[test]
    fn none_emits_no_system_prompt_flag() {
        let argv = build_argv(&Options::default()).expect("build argv");
        assert!(!argv.iter().any(|a| a == "--system-prompt"));
        assert!(!argv.iter().any(|a| a == "--append-system-prompt"));
    }

    fn agents_flag_value(argv: &[String]) -> Option<&str> {
        let idx = argv.iter().position(|a| a == "--agents")?;
        argv.get(idx + 1).map(String::as_str)
    }

    #[test]
    fn omits_agents_flag_when_map_empty() {
        let argv = build_argv(&Options::builder().build()).expect("build argv");
        assert!(agents_flag_value(&argv).is_none());
    }

    #[test]
    fn serializes_agents_map_to_json_flag() {
        use crate::agent::subagents::AgentDefinition;

        let reviewer = AgentDefinition::new("reviewer", "be careful")
            .expect("valid")
            .with_model(ModelId::claude_haiku_4_5());
        let opts = Options::builder().agent("reviewer", reviewer).build();
        let argv = build_argv(&opts).expect("build argv");
        let json = agents_flag_value(&argv).expect("--agents present");
        let parsed: serde_json::Value = serde_json::from_str(json).expect("valid json");
        assert_eq!(parsed["reviewer"]["description"], "reviewer");
        assert_eq!(parsed["reviewer"]["prompt"], "be careful");
        assert_eq!(parsed["reviewer"]["model"], "claude-haiku-4-5");
    }

    #[test]
    fn new_session_emits_no_session_flags() {
        let argv = build_argv(&Options::default()).expect("build argv");
        assert!(!argv.iter().any(|a| a == "--continue"));
        assert!(!argv.iter().any(|a| a == "--resume"));
        assert!(!argv.iter().any(|a| a == "--fork-session"));
    }

    #[test]
    fn continue_without_fork_emits_continue_only() {
        use crate::agent::types::SessionControl;
        let opts = Options::builder()
            .session(SessionControl::Continue { fork: false })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        assert!(argv.iter().any(|a| a == "--continue"));
        assert!(!argv.iter().any(|a| a == "--fork-session"));
    }

    #[test]
    fn continue_with_fork_emits_continue_and_fork() {
        use crate::agent::types::SessionControl;
        let opts = Options::builder()
            .session(SessionControl::Continue { fork: true })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        assert!(argv.iter().any(|a| a == "--continue"));
        assert!(argv.iter().any(|a| a == "--fork-session"));
    }

    #[test]
    fn resume_without_fork_emits_resume_and_id() {
        use crate::agent::types::{SessionControl, SessionId};
        let opts = Options::builder()
            .session(SessionControl::Resume {
                id: SessionId::new("sess_42"),
                fork: false,
                resume_at: None,
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let idx = argv
            .iter()
            .position(|a| a == "--resume")
            .expect("--resume present");
        assert_eq!(argv.get(idx + 1).map(String::as_str), Some("sess_42"));
        assert!(!argv.iter().any(|a| a == "--fork-session"));
    }

    #[test]
    fn resume_with_fork_emits_resume_id_and_fork() {
        use crate::agent::types::{SessionControl, SessionId};
        let opts = Options::builder()
            .session(SessionControl::Resume {
                id: SessionId::new("sess_99"),
                fork: true,
                resume_at: None,
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let idx = argv
            .iter()
            .position(|a| a == "--resume")
            .expect("--resume present");
        assert_eq!(argv.get(idx + 1).map(String::as_str), Some("sess_99"));
        assert!(argv.iter().any(|a| a == "--fork-session"));
    }

    #[test]
    fn resume_at_lowers_to_equals_flag_alongside_resume() {
        use crate::agent::types::{MessageId, SessionControl, SessionId};
        let opts = Options::builder()
            .session(SessionControl::Resume {
                id: SessionId::new("sess_1"),
                fork: false,
                resume_at: Some(MessageId::new("m-uuid").expect("valid")),
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        assert!(argv.iter().any(|a| a == "--resume"));
        assert!(
            argv.iter().any(|a| a == "--resume-session-at=m-uuid"),
            "got: {}",
            argv.join(" ")
        );
    }

    #[test]
    fn no_resume_at_emits_no_flag() {
        use crate::agent::types::{SessionControl, SessionId};
        let opts = Options::builder()
            .session(SessionControl::Resume {
                id: SessionId::new("sess_1"),
                fork: false,
                resume_at: None,
            })
            .build();
        assert!(
            !build_argv(&opts)
                .expect("build argv")
                .iter()
                .any(|a| a.starts_with("--resume-session-at")),
            "resume_at None must emit no flag"
        );
    }

    #[test]
    fn resume_at_coexists_with_fork() {
        use crate::agent::types::{MessageId, SessionControl, SessionId};
        let opts = Options::builder()
            .session(SessionControl::Resume {
                id: SessionId::new("sess_1"),
                fork: true,
                resume_at: Some(MessageId::new("m2").expect("valid")),
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        assert!(argv.iter().any(|a| a == "--fork-session"));
        assert!(argv.iter().any(|a| a == "--resume-session-at=m2"));
    }

    #[test]
    fn lowers_fallback_model() {
        use crate::types::ModelId;
        let opts = Options::builder()
            .fallback_model(ModelId::custom("claude-haiku-4-5").expect("model"))
            .build();
        let joined = build_argv(&opts).expect("build argv").join(" ");
        assert!(
            joined.contains("--fallback-model claude-haiku-4-5"),
            "got: {joined}"
        );
    }

    #[test]
    fn strict_mcp_config_emits_presence_flag_only_when_true() {
        assert!(
            !build_argv(&Options::default())
                .expect("build argv")
                .iter()
                .any(|a| a == "--strict-mcp-config")
        );
        let opts = Options::builder().strict_mcp_config(true).build();
        assert!(
            build_argv(&opts)
                .expect("build argv")
                .iter()
                .any(|a| a == "--strict-mcp-config")
        );
    }

    #[test]
    fn add_dirs_emit_one_flag_with_each_directory_as_a_value() {
        let opts = Options::builder().add_dir("/a").add_dir("/b").build();
        let argv = build_argv(&opts).expect("build argv");
        let idx = argv
            .iter()
            .position(|a| a == "--add-dir")
            .expect("--add-dir present");
        assert_eq!(argv.get(idx + 1).map(String::as_str), Some("/a"));
        assert_eq!(argv.get(idx + 2).map(String::as_str), Some("/b"));
        // Exactly one --add-dir flag (variadic form), not one per directory.
        assert_eq!(argv.iter().filter(|a| *a == "--add-dir").count(), 1);
    }

    #[test]
    fn settings_path_and_inline_lower_to_the_flag_argument() {
        let p = Options::builder().settings_path("/etc/s.json").build();
        let pa = build_argv(&p).expect("build argv");
        let pi = pa
            .iter()
            .position(|a| a == "--settings")
            .expect("--settings");
        assert_eq!(pa.get(pi + 1).map(String::as_str), Some("/etc/s.json"));

        let i = Options::builder()
            .settings_inline(serde_json::json!({ "k": 1 }))
            .build();
        let ia = build_argv(&i).expect("build argv");
        let ii = ia
            .iter()
            .position(|a| a == "--settings")
            .expect("--settings");
        assert_eq!(ia.get(ii + 1).map(String::as_str), Some("{\"k\":1}"));
    }

    #[test]
    fn max_budget_usd_lowers_to_amount() {
        use crate::agent::types::BudgetUsd;
        let opts = Options::builder()
            .max_budget_usd(BudgetUsd::new(5.0).expect("positive"))
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let idx = argv
            .iter()
            .position(|a| a == "--max-budget-usd")
            .expect("flag");
        assert_eq!(argv.get(idx + 1).map(String::as_str), Some("5"));
    }

    #[test]
    fn include_partial_messages_emits_presence_flag() {
        assert!(
            !build_argv(&Options::default())
                .expect("build argv")
                .iter()
                .any(|a| a == "--include-partial-messages")
        );
        let opts = Options::builder().include_partial_messages(true).build();
        assert!(
            build_argv(&opts)
                .expect("build argv")
                .iter()
                .any(|a| a == "--include-partial-messages")
        );
    }

    #[test]
    fn permission_prompt_tool_name_overrides_stdio_sentinel() {
        let opts = Options::builder()
            .permission_prompt_tool_name("mcp__gate__approve")
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let idx = argv
            .iter()
            .position(|a| a == "--permission-prompt-tool")
            .expect("flag present");
        assert_eq!(
            argv.get(idx + 1).map(String::as_str),
            Some("mcp__gate__approve")
        );
    }

    #[test]
    fn permission_prompt_tool_name_wins_over_active_policy() {
        use crate::agent::error::AgentError;
        use crate::agent::permissions::{PermissionContext, PermissionDecision, PermissionPolicy};
        use std::sync::Arc;

        struct P;
        #[async_trait::async_trait]
        impl PermissionPolicy for P {
            async fn can_use_tool(
                &self,
                _t: &str,
                _i: &serde_json::Value,
                _c: PermissionContext,
                _cancel: crate::agent::cancel::CancelSignal,
            ) -> Result<PermissionDecision, AgentError> {
                Ok(PermissionDecision::allow())
            }
        }

        let opts = Options::builder()
            .permission_policy(Arc::new(P))
            .permission_prompt_tool_name("mcp__gate__approve")
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let joined = argv.join(" ");
        assert!(
            joined.contains("--permission-prompt-tool mcp__gate__approve"),
            "got: {joined}"
        );
        assert!(
            !joined.contains("--permission-prompt-tool stdio"),
            "got: {joined}"
        );
    }

    #[test]
    fn custom_prompt_tool_emits_even_without_a_policy() {
        // A caller-owned prompt tool is emitted regardless of permission_policy.
        let opts = Options::builder()
            .permission_prompt_tool_name("mcp__gate__approve")
            .build();
        assert!(
            build_argv(&opts)
                .expect("build argv")
                .iter()
                .any(|a| a == "--permission-prompt-tool")
        );
    }

    #[test]
    fn include_hook_events_emits_presence_flag() {
        assert!(
            !build_argv(&Options::default())
                .expect("build argv")
                .iter()
                .any(|a| a == "--include-hook-events")
        );
        let opts = Options::builder().include_hook_events(true).build();
        assert!(
            build_argv(&opts)
                .expect("build argv")
                .iter()
                .any(|a| a == "--include-hook-events")
        );
    }

    #[test]
    fn session_id_emits_flag_for_a_new_session() {
        use crate::agent::types::SessionId;
        let opts = Options::builder()
            .session_id(SessionId::new("sess_7"))
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let idx = argv
            .iter()
            .position(|a| a == "--session-id")
            .expect("--session-id present");
        assert_eq!(argv.get(idx + 1).map(String::as_str), Some("sess_7"));
    }

    #[test]
    fn session_id_is_omitted_when_resuming_or_continuing() {
        use crate::agent::types::{SessionControl, SessionId};

        let resuming = Options::builder()
            .session_id(SessionId::new("sess_7"))
            .session(SessionControl::Resume {
                id: SessionId::new("sess_1"),
                fork: false,
                resume_at: None,
            })
            .build();
        let resuming_argv = build_argv(&resuming).expect("build argv");
        assert!(!resuming_argv.iter().any(|a| a == "--session-id"));
        let idx = resuming_argv
            .iter()
            .position(|a| a == "--resume")
            .expect("--resume present");
        assert_eq!(
            resuming_argv.get(idx + 1).map(String::as_str),
            Some("sess_1")
        );

        let continuing = Options::builder()
            .session_id(SessionId::new("sess_7"))
            .session(SessionControl::Continue { fork: false })
            .build();
        let continuing_argv = build_argv(&continuing).expect("build argv");
        assert!(!continuing_argv.iter().any(|a| a == "--session-id"));
        assert!(continuing_argv.iter().any(|a| a == "--continue"));
    }

    #[test]
    fn omits_session_id_flag_when_unset() {
        assert!(
            !build_argv(&Options::default())
                .expect("build argv")
                .iter()
                .any(|a| a == "--session-id")
        );
    }

    #[test]
    fn session_persistence_emits_flag_only_when_disabled() {
        use crate::agent::types::SessionPersistence;

        // Default (Enabled) emits nothing.
        assert!(
            !build_argv(&Options::default())
                .expect("build argv")
                .iter()
                .any(|a| a == "--no-session-persistence")
        );

        let opts = Options::builder()
            .session_persistence(SessionPersistence::Disabled)
            .build();
        assert!(
            build_argv(&opts)
                .expect("build argv")
                .iter()
                .any(|a| a == "--no-session-persistence")
        );
    }

    #[test]
    fn lowers_effort_when_set_and_omits_when_none() {
        use crate::agent::types::EffortLevel;

        let with =
            build_argv(&Options::builder().effort(EffortLevel::High).build()).expect("build argv");
        assert!(with.join(" ").contains("--effort high"));

        let without = build_argv(&Options::default()).expect("build argv");
        assert!(!without.join(" ").contains("--effort"));
    }

    #[test]
    fn lowers_setting_sources_as_csv_equals_form() {
        use crate::agent::types::SettingSource;
        let opts = Options::builder()
            .setting_sources([
                SettingSource::User,
                SettingSource::Project,
                SettingSource::Local,
            ])
            .build();
        let argv = build_argv(&opts).expect("build argv");
        assert!(
            argv.contains(&"--setting-sources=user,project,local".to_string()),
            "got: {argv:?}"
        );
    }

    #[test]
    fn omits_setting_sources_when_empty() {
        let argv = build_argv(&Options::default()).expect("build argv");
        assert!(
            !argv.iter().any(|a| a.starts_with("--setting-sources")),
            "got: {argv:?}"
        );
    }

    #[test]
    fn lowers_betas_as_space_form_comma_joined() {
        let opts = Options::builder()
            .betas(["a".to_string(), "b".to_string()])
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv
            .iter()
            .position(|a| a == "--betas")
            .expect("--betas present");
        assert_eq!(argv[i + 1], "a,b");
    }

    #[test]
    fn omits_betas_when_empty() {
        let argv = build_argv(&Options::default()).expect("build argv");
        assert!(!argv.iter().any(|a| a == "--betas"), "got: {argv:?}");
    }

    #[test]
    fn lowers_plugins_choosing_flag_by_skip_mcp_discovery() {
        use crate::agent::types::PluginSpec;
        let opts = Options::builder()
            .plugin(PluginSpec {
                path: "/one".into(),
                skip_mcp_discovery: false,
            })
            .plugin(PluginSpec {
                path: "/two".into(),
                skip_mcp_discovery: true,
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let joined = argv.join(" ");
        assert!(joined.contains("--plugin-dir /one"), "got: {joined}");
        assert!(joined.contains("--plugin-dir-no-mcp /two"), "got: {joined}");
    }

    #[test]
    fn lowers_thinking_enabled_with_budget_to_max_thinking_tokens() {
        use crate::agent::types::{ThinkingConfig, ThinkingMode};
        let opts = Options::builder()
            .thinking(ThinkingConfig {
                mode: ThinkingMode::Enabled {
                    budget_tokens: Some(2048),
                },
                display: None,
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv
            .iter()
            .position(|a| a == "--max-thinking-tokens")
            .expect("flag");
        assert_eq!(argv[i + 1], "2048");
        assert!(!argv.iter().any(|a| a == "--thinking"), "got: {argv:?}");
    }

    #[test]
    fn lowers_thinking_adaptive_and_enabled_without_budget_to_adaptive() {
        use crate::agent::types::{ThinkingConfig, ThinkingMode};
        for mode in [
            ThinkingMode::Adaptive,
            ThinkingMode::Enabled {
                budget_tokens: None,
            },
        ] {
            let opts = Options::builder()
                .thinking(ThinkingConfig {
                    mode,
                    display: None,
                })
                .build();
            let argv = build_argv(&opts).expect("build argv");
            let i = argv.iter().position(|a| a == "--thinking").expect("flag");
            assert_eq!(argv[i + 1], "adaptive");
        }
    }

    #[test]
    fn lowers_thinking_disabled_and_suppresses_display() {
        use crate::agent::types::{ThinkingConfig, ThinkingDisplay, ThinkingMode};
        let opts = Options::builder()
            .thinking(ThinkingConfig {
                mode: ThinkingMode::Disabled,
                display: Some(ThinkingDisplay::Summarized),
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv.iter().position(|a| a == "--thinking").expect("flag");
        assert_eq!(argv[i + 1], "disabled");
        assert!(
            !argv.iter().any(|a| a == "--thinking-display"),
            "display suppressed under disabled: {argv:?}"
        );
    }

    #[test]
    fn lowers_thinking_display_when_not_disabled() {
        use crate::agent::types::{ThinkingConfig, ThinkingDisplay, ThinkingMode};
        let opts = Options::builder()
            .thinking(ThinkingConfig {
                mode: ThinkingMode::Adaptive,
                display: Some(ThinkingDisplay::Omitted),
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv
            .iter()
            .position(|a| a == "--thinking-display")
            .expect("flag");
        assert_eq!(argv[i + 1], "omitted");
    }

    #[test]
    fn scalar_max_thinking_tokens_is_fallback_and_zero_disables() {
        let disabled =
            build_argv(&Options::builder().max_thinking_tokens(0).build()).expect("build argv");
        let di = disabled
            .iter()
            .position(|a| a == "--thinking")
            .expect("flag");
        assert_eq!(disabled[di + 1], "disabled");

        let budget =
            build_argv(&Options::builder().max_thinking_tokens(4096).build()).expect("build argv");
        let bi = budget
            .iter()
            .position(|a| a == "--max-thinking-tokens")
            .expect("flag");
        assert_eq!(budget[bi + 1], "4096");
    }

    #[test]
    fn thinking_object_wins_over_scalar() {
        use crate::agent::types::{ThinkingConfig, ThinkingMode};
        let opts = Options::builder()
            .thinking(ThinkingConfig {
                mode: ThinkingMode::Disabled,
                display: None,
            })
            .max_thinking_tokens(4096)
            .build();
        let argv = build_argv(&opts).expect("build argv");
        assert!(
            !argv.iter().any(|a| a == "--max-thinking-tokens"),
            "scalar ignored when object set: {argv:?}"
        );
        let i = argv.iter().position(|a| a == "--thinking").expect("flag");
        assert_eq!(argv[i + 1], "disabled");
    }

    #[test]
    fn lowers_agent_selector_and_coexists_with_plural_agents() {
        use crate::agent::subagents::AgentDefinition;
        let opts = Options::builder()
            .agent_name("reviewer")
            .agent(
                "helper",
                AgentDefinition::new("desc", "prompt").expect("agent def"),
            )
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv
            .iter()
            .position(|a| a == "--agent")
            .expect("--agent present");
        assert_eq!(argv[i + 1], "reviewer");
        assert!(
            argv.iter().any(|a| a == "--agents"),
            "plural --agents still emitted: {argv:?}"
        );
    }

    #[test]
    fn omits_agent_selector_when_unset() {
        let argv = build_argv(&Options::default()).expect("build argv");
        assert!(!argv.iter().any(|a| a == "--agent"), "got: {argv:?}");
    }

    #[test]
    fn skills_all_appends_bare_skill_tool() {
        use crate::agent::types::Skills;
        let opts = Options::builder().skills(Skills::All).build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv
            .iter()
            .position(|a| a == "--allowed-tools")
            .expect("flag");
        assert_eq!(argv[i + 1], "Skill");
    }

    #[test]
    fn skills_list_appends_named_skill_tools_after_existing_allowed() {
        use crate::agent::types::Skills;
        let opts = Options::builder()
            .allowed_tools(vec!["Bash".into()])
            .skills(Skills::List(vec!["pdf".into(), "xlsx".into()]))
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv
            .iter()
            .position(|a| a == "--allowed-tools")
            .expect("flag");
        assert_eq!(argv[i + 1], "Bash,Skill(pdf),Skill(xlsx)");
    }

    #[test]
    fn skills_entries_dedup_against_existing_allowed_tools() {
        use crate::agent::types::Skills;
        let opts = Options::builder()
            .allowed_tools(vec!["Skill(pdf)".into()])
            .skills(Skills::List(vec!["pdf".into(), "xlsx".into()]))
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv
            .iter()
            .position(|a| a == "--allowed-tools")
            .expect("flag");
        assert_eq!(argv[i + 1], "Skill(pdf),Skill(xlsx)");
    }

    #[test]
    fn sandbox_alone_emits_settings_with_sandbox_key_and_defaults_fail_if_unavailable() {
        use crate::agent::types::SandboxConfig;
        let opts = Options::builder()
            .sandbox(SandboxConfig {
                enabled: Some(true),
                ..Default::default()
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv.iter().position(|a| a == "--settings").expect("flag");
        let v: serde_json::Value = serde_json::from_str(&argv[i + 1]).expect("json");
        assert_eq!(
            v,
            serde_json::json!({ "sandbox": { "enabled": true, "failIfUnavailable": true } })
        );
    }

    #[test]
    fn sandbox_merges_into_inline_settings() {
        use crate::agent::types::SandboxConfig;
        let opts = Options::builder()
            .settings_inline(serde_json::json!({ "theme": "dark" }))
            .sandbox(SandboxConfig {
                enabled: Some(false),
                ..Default::default()
            })
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv.iter().position(|a| a == "--settings").expect("flag");
        let v: serde_json::Value = serde_json::from_str(&argv[i + 1]).expect("json");
        assert_eq!(
            v,
            serde_json::json!({ "theme": "dark", "sandbox": { "enabled": false } })
        );
    }

    #[test]
    fn sandbox_with_settings_path_is_an_error() {
        use crate::agent::error::AgentError;
        use crate::agent::types::SandboxConfig;
        let opts = Options::builder()
            .settings_path("/etc/claude/settings.json")
            .sandbox(SandboxConfig {
                enabled: Some(true),
                ..Default::default()
            })
            .build();
        let err = build_argv(&opts).expect_err("conflict");
        assert!(matches!(err, AgentError::SandboxWithSettingsPath));
    }

    #[test]
    fn settings_path_alone_is_unchanged() {
        let opts = Options::builder()
            .settings_path("/etc/claude/settings.json")
            .build();
        let argv = build_argv(&opts).expect("build argv");
        let i = argv.iter().position(|a| a == "--settings").expect("flag");
        assert_eq!(argv[i + 1], "/etc/claude/settings.json");
    }
}
