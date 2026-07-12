//! Mapping session options to the backend's argument vector.

use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;
use crate::agent::system_prompt::SystemPromptConfig;
use crate::agent::types::SessionControl;

/// Build the full argument vector for spawning the backend.
///
/// Caller-supplied `executable_args` come first, then the SDK-managed
/// stream-protocol flags, then mapped option fields. `cwd` and `env` are not
/// argv — they are applied to the process spawn config.
pub(super) fn build_argv(options: &Options) -> Vec<String> {
    let mut argv: Vec<String> = options.executable_args.clone();

    argv.push("--output-format".to_string());
    argv.push("stream-json".to_string());
    argv.push("--input-format".to_string());
    argv.push("stream-json".to_string());
    argv.push("--verbose".to_string());

    argv.push("--permission-mode".to_string());
    argv.push(permission_mode_wire(options.permission_mode).to_string());

    // A registered policy routes tool-permission prompts over the control
    // protocol; the `stdio` sentinel selects that path.
    if options.permission_policy.is_some() {
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
    if !options.allowed_tools.is_empty() {
        argv.push("--allowed-tools".to_string());
        argv.push(options.allowed_tools.join(","));
    }
    if !options.disallowed_tools.is_empty() {
        argv.push("--disallowed-tools".to_string());
        argv.push(options.disallowed_tools.join(","));
    }
    if let Some(max_turns) = options.max_turns {
        argv.push("--max-turns".to_string());
        argv.push(max_turns.to_string());
    }
    for server in &options.mcp_servers {
        argv.push("--mcp-config".to_string());
        let config = serde_json::json!({ server.name(): server.config() });
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
    argv.extend(session_args(&options.session));
    argv
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

/// Map the session intent to the backend's session flags.
///
/// `--fork-session` combines with either `--continue` or `--resume <id>`.
/// `New` emits nothing; the binary starts a fresh session by default.
fn session_args(control: &SessionControl) -> Vec<String> {
    match control {
        SessionControl::New => Vec::new(),
        SessionControl::Continue { fork } => {
            let mut args = vec!["--continue".to_string()];
            if *fork {
                args.push("--fork-session".to_string());
            }
            args
        }
        SessionControl::Resume { id, fork } => {
            let mut args = vec!["--resume".to_string(), id.as_str().to_string()];
            if *fork {
                args.push("--fork-session".to_string());
            }
            args
        }
    }
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
        let argv = build_argv(&Options::default());
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
        let argv = build_argv(&opts);
        assert_eq!(argv.first().map(String::as_str), Some("--mcp-debug"));
        let joined = argv.join(" ");
        assert!(joined.contains("--model claude-sonnet-4-5"));
        assert!(joined.contains("--system-prompt be brief"));
        assert!(joined.contains("--permission-mode acceptEdits"));
        assert!(joined.contains("--allowed-tools Bash,Read"));
        assert!(joined.contains("--disallowed-tools Write"));
        assert!(joined.contains("--max-turns 5"));
        assert!(joined.contains("--mcp-config"));
        assert!(joined.contains("\"fs\""));
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
        let argv = build_argv(&Options::default());
        assert!(!argv.iter().any(|a| a == "--permission-prompt-tool"));
    }

    #[test]
    fn emits_mcp_config_type_sdk_for_in_process_server() {
        use crate::agent::mcp::SdkMcpServer;
        let opts = Options::builder()
            .sdk_mcp_server(SdkMcpServer::builder("calc").build())
            .build();
        let argv = build_argv(&opts);
        let joined = argv.join(" ");
        assert!(joined.contains("--mcp-config"), "got: {joined}");
        // The declaration carries the server name and the sdk type marker.
        assert!(joined.contains("\"calc\""), "got: {joined}");
        assert!(joined.contains("\"type\":\"sdk\""), "got: {joined}");
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
            ) -> Result<PermissionDecision, AgentError> {
                Ok(PermissionDecision::allow())
            }
        }

        let opts = Options::builder().permission_policy(Arc::new(P)).build();
        let argv = build_argv(&opts);
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
        let argv = build_argv(&opts);
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
        let argv = build_argv(&Options::default());
        assert!(!argv.iter().any(|a| a == "--system-prompt"));
        assert!(!argv.iter().any(|a| a == "--append-system-prompt"));
    }

    fn agents_flag_value(argv: &[String]) -> Option<&str> {
        let idx = argv.iter().position(|a| a == "--agents")?;
        argv.get(idx + 1).map(String::as_str)
    }

    #[test]
    fn omits_agents_flag_when_map_empty() {
        let argv = build_argv(&Options::builder().build());
        assert!(agents_flag_value(&argv).is_none());
    }

    #[test]
    fn serializes_agents_map_to_json_flag() {
        use crate::agent::subagents::AgentDefinition;

        let reviewer = AgentDefinition::new("reviewer", "be careful")
            .expect("valid")
            .with_model(ModelId::claude_haiku_4_5());
        let opts = Options::builder().agent("reviewer", reviewer).build();
        let argv = build_argv(&opts);
        let json = agents_flag_value(&argv).expect("--agents present");
        let parsed: serde_json::Value = serde_json::from_str(json).expect("valid json");
        assert_eq!(parsed["reviewer"]["description"], "reviewer");
        assert_eq!(parsed["reviewer"]["prompt"], "be careful");
        assert_eq!(parsed["reviewer"]["model"], "claude-haiku-4-5");
    }

    #[test]
    fn new_session_emits_no_session_flags() {
        let argv = build_argv(&Options::default());
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
        let argv = build_argv(&opts);
        assert!(argv.iter().any(|a| a == "--continue"));
        assert!(!argv.iter().any(|a| a == "--fork-session"));
    }

    #[test]
    fn continue_with_fork_emits_continue_and_fork() {
        use crate::agent::types::SessionControl;
        let opts = Options::builder()
            .session(SessionControl::Continue { fork: true })
            .build();
        let argv = build_argv(&opts);
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
            })
            .build();
        let argv = build_argv(&opts);
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
            })
            .build();
        let argv = build_argv(&opts);
        let idx = argv
            .iter()
            .position(|a| a == "--resume")
            .expect("--resume present");
        assert_eq!(argv.get(idx + 1).map(String::as_str), Some("sess_99"));
        assert!(argv.iter().any(|a| a == "--fork-session"));
    }
}
