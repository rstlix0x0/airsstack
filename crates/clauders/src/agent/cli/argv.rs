//! Mapping session options to the backend's argument vector.

use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;

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
    if let Some(system_prompt) = &options.system_prompt {
        argv.push("--system-prompt".to_string());
        argv.push(system_prompt.clone());
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
    argv
}

/// The backend's camelCase wire spelling for a permission mode.
pub(super) const fn permission_mode_wire(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Default => "default",
        PermissionMode::AcceptEdits => "acceptEdits",
        PermissionMode::Plan => "plan",
        PermissionMode::BypassPermissions => "bypassPermissions",
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
    }

    #[test]
    fn omits_permission_prompt_tool_without_policy() {
        let argv = build_argv(&Options::default());
        assert!(!argv.iter().any(|a| a == "--permission-prompt-tool"));
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
                Ok(PermissionDecision::Allow {
                    updated_input: None,
                })
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
}
