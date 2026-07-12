//! The native subagent surface for `ApiRuntime`.
//!
//! Pure, loop-independent pieces: the built-in `Agent` tool declaration the
//! parent loop offers when subagents are registered, the predicate that
//! recognizes an `Agent`/`Task` tool call, the parser for its
//! `subagent_type`/`prompt` input, and [`ToolFilter`] — which narrows a
//! declared tool list to a subagent's allowed/disallowed set. The loop
//! integration (`run_subagent`) lives with `drive` in `runtime.rs`.

use std::collections::HashMap;

use crate::agent::subagents::AgentDefinition;
use crate::messages::tools::Tool as WireTool;
use crate::types::ToolName;

/// The name the parent loop declares and the model emits to invoke a
/// subagent (renamed from `Task` upstream; both are accepted on input).
pub(super) const AGENT_TOOL_NAME: &str = "Agent";

/// Whether `name` is the subagent-invocation tool. Accepts the current
/// `Agent` spelling and the legacy `Task` spelling for forward-compat.
pub(super) fn is_agent_tool(name: &str) -> bool {
    name == AGENT_TOOL_NAME || name == "Task"
}

/// Read `subagent_type` and `prompt` from an `Agent` tool call's input.
/// Returns `None` if either string field is absent.
pub(super) fn parse_agent_call(input: &serde_json::Value) -> Option<(String, String)> {
    let subagent_type = input.get("subagent_type")?.as_str()?.to_string();
    let prompt = input.get("prompt")?.as_str()?.to_string();
    Some((subagent_type, prompt))
}

/// Declare the built-in `Agent` tool whose `subagent_type` is constrained to
/// the registered agent names. Returns `None` only if the fixed tool name is
/// somehow invalid (never in practice); callers skip declaration then.
pub(super) fn declare_agent_tool(agents: &HashMap<String, AgentDefinition>) -> Option<WireTool> {
    let name = ToolName::new(AGENT_TOOL_NAME).ok()?;
    let mut names: Vec<&str> = agents.keys().map(String::as_str).collect();
    names.sort_unstable();
    Some(WireTool {
        name,
        description: "Delegate a subtask to a named subagent. Provide subagent_type (one of the \
             registered agents) and a self-contained prompt; the subagent runs in isolation \
             and returns only its final message."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "subagent_type": { "type": "string", "enum": names },
                "prompt": { "type": "string" }
            },
            "required": ["subagent_type", "prompt"]
        }),
        cache_control: None,
        strict: None,
    })
}

/// Narrows a declared tool list to a subagent's effective set: an optional
/// allowlist (`None` inherits all) minus a denylist (exact names and MCP
/// patterns `mcp__server`, `mcp__server__*`, `mcp__*`).
#[derive(Clone, Debug, Default)]
pub(super) struct ToolFilter {
    allow: Option<Vec<String>>,
    deny: Vec<String>,
}

impl ToolFilter {
    /// The identity filter: inherit every declared tool (parent loop).
    pub(super) const fn inherit_all() -> Self {
        Self {
            allow: None,
            deny: Vec::new(),
        }
    }

    /// The filter for a subagent: its `tools` allowlist and
    /// `disallowed_tools` denylist.
    pub(super) fn from_definition(def: &AgentDefinition) -> Self {
        Self {
            allow: def.tools().map(<[String]>::to_vec),
            deny: def.disallowed_tools().to_vec(),
        }
    }

    /// Keep only the tools this filter admits.
    pub(super) fn apply(&self, tools: Vec<WireTool>) -> Vec<WireTool> {
        tools
            .into_iter()
            .filter(|t| self.admits(t.name.as_str()))
            .collect()
    }

    fn admits(&self, name: &str) -> bool {
        if let Some(allow) = &self.allow {
            if !allow.iter().any(|a| a == name) {
                return false;
            }
        }
        !self.deny.iter().any(|d| deny_matches(d, name))
    }
}

/// Whether a denylist `pattern` removes tool `name`.
fn deny_matches(pattern: &str, name: &str) -> bool {
    if pattern == name {
        return true;
    }
    if pattern == "mcp__*" {
        return name.starts_with("mcp__");
    }
    if let Some(prefix) = pattern.strip_suffix("__*") {
        // Re-attach the `__` boundary so `mcp__fs__*` matches only tools of the
        // `fs` server (`mcp__fs__<tool>`) and not a prefix-colliding server such
        // as `mcp__fsxyz__<tool>`.
        return name.starts_with(&format!("{prefix}__"));
    }
    if pattern.starts_with("mcp__") {
        return name.starts_with(&format!("{pattern}__"));
    }
    false
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::collections::HashMap;

    use super::{ToolFilter, declare_agent_tool, is_agent_tool, parse_agent_call};
    use crate::agent::subagents::AgentDefinition;
    use crate::messages::tools::Tool as WireTool;
    use crate::types::ToolName;

    fn wire(name: &str) -> WireTool {
        WireTool {
            name: ToolName::new(name).expect("name"),
            description: String::new(),
            input_schema: serde_json::json!({}),
            cache_control: None,
            strict: None,
        }
    }

    fn names(tools: &[WireTool]) -> Vec<&str> {
        tools.iter().map(|t| t.name.as_str()).collect()
    }

    #[test]
    fn recognizes_agent_and_task_names() {
        assert!(is_agent_tool("Agent"));
        assert!(is_agent_tool("Task"));
        assert!(!is_agent_tool("mcp__calc__add"));
    }

    #[test]
    fn parses_subagent_type_and_prompt() {
        let input = serde_json::json!({ "subagent_type": "reviewer", "prompt": "check it" });
        assert_eq!(
            parse_agent_call(&input),
            Some(("reviewer".to_string(), "check it".to_string()))
        );
        assert_eq!(
            parse_agent_call(&serde_json::json!({ "prompt": "x" })),
            None
        );
        assert_eq!(
            parse_agent_call(&serde_json::json!({ "subagent_type": "r" })),
            None
        );
    }

    #[test]
    fn declares_agent_tool_with_sorted_enum_of_names() {
        let mut agents = HashMap::new();
        agents.insert(
            "zeta".to_string(),
            AgentDefinition::new("z", "z").expect("valid"),
        );
        agents.insert(
            "alpha".to_string(),
            AgentDefinition::new("a", "a").expect("valid"),
        );
        let tool = declare_agent_tool(&agents).expect("declared");
        assert_eq!(tool.name.as_str(), "Agent");
        assert_eq!(
            tool.input_schema["properties"]["subagent_type"]["enum"],
            serde_json::json!(["alpha", "zeta"])
        );
        assert_eq!(
            tool.input_schema["required"],
            serde_json::json!(["subagent_type", "prompt"])
        );
    }

    #[test]
    fn inherit_all_keeps_every_tool() {
        let tools = vec![wire("mcp__calc__add"), wire("mcp__fs__read")];
        let kept = ToolFilter::inherit_all().apply(tools);
        assert_eq!(names(&kept), vec!["mcp__calc__add", "mcp__fs__read"]);
    }

    #[test]
    fn allowlist_restricts_to_listed_names() {
        let def = AgentDefinition::new("d", "p")
            .expect("valid")
            .with_tools(vec!["mcp__calc__add".to_string()]);
        let kept = ToolFilter::from_definition(&def)
            .apply(vec![wire("mcp__calc__add"), wire("mcp__fs__read")]);
        assert_eq!(names(&kept), vec!["mcp__calc__add"]);
    }

    #[test]
    fn denylist_honors_exact_and_mcp_patterns() {
        let def = AgentDefinition::new("d", "p")
            .expect("valid")
            .with_disallowed_tools(vec!["mcp__fs__*".to_string()]);
        let kept = ToolFilter::from_definition(&def).apply(vec![
            wire("mcp__calc__add"),
            wire("mcp__fs__read"),
            wire("mcp__fs__write"),
        ]);
        assert_eq!(names(&kept), vec!["mcp__calc__add"]);

        let def_all = AgentDefinition::new("d", "p")
            .expect("valid")
            .with_disallowed_tools(vec!["mcp__*".to_string()]);
        let kept = ToolFilter::from_definition(&def_all)
            .apply(vec![wire("mcp__calc__add"), wire("plain")]);
        assert_eq!(names(&kept), vec!["plain"]);
    }

    #[test]
    fn denylist_server_glob_respects_the_name_boundary() {
        // `mcp__fs__*` targets the `fs` server only; a prefix-colliding server
        // (`mcp__fsxyz`) must survive.
        let def = AgentDefinition::new("d", "p")
            .expect("valid")
            .with_disallowed_tools(vec!["mcp__fs__*".to_string()]);
        let kept = ToolFilter::from_definition(&def)
            .apply(vec![wire("mcp__fs__read"), wire("mcp__fsxyz__read")]);
        assert_eq!(names(&kept), vec!["mcp__fsxyz__read"]);
    }
}
