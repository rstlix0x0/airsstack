//! The canonical programmatic subagent definition.
//!
//! An [`AgentDefinition`] describes one named helper agent the running agent
//! can delegate a subtask to. Required `description`/`prompt` are validated
//! non-empty at construction (parse-don't-validate); every other field is
//! optional and `None`/empty means "inherit from the parent agent".

use serde::Serialize;
use thiserror::Error;

use crate::agent::permissions::PermissionMode;
use crate::agent::types::{EffortLevel, McpServerConfig};
use crate::types::ModelId;

use super::MemorySource;

/// Reasons [`AgentDefinition::new`] can reject input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum AgentDefinitionError {
    /// The description was empty or whitespace-only.
    #[error("agent definition description must not be empty")]
    EmptyDescription,
    /// The prompt was empty or whitespace-only.
    #[error("agent definition prompt must not be empty")]
    EmptyPrompt,
}

/// A named helper agent the running agent can delegate a subtask to.
///
/// `description` tells the model when to choose this agent; `prompt` is the
/// agent's own system prompt. Optional fields override the parent when set:
/// `tools` restricts the tool set (`None` inherits all), `disallowed_tools`
/// subtracts from it, `model` is the per-subtask model override (`None`
/// inherits the parent model), `max_turns` and `permission_mode` likewise
/// inherit when unset.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    description: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    disallowed_tools: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<ModelId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    permission_mode: Option<PermissionMode>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<MemorySource>,
    /// External MCP servers scoped to this agent. **Unconfirmed wire shape:**
    /// the official `AgentMcpServerSpec` element is undocumented, so each server
    /// serializes as the assumed `{ "name": …, "config": … }` — the official
    /// element likely inlines transport fields instead. Re-verify against a live
    /// `--agents` round-trip before treating `mcpServers` as at parity.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    mcp_servers: Vec<McpServerConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effort: Option<EffortLevel>,
}

impl AgentDefinition {
    /// Construct a definition, trimming and validating the required fields.
    ///
    /// # Errors
    /// [`AgentDefinitionError::EmptyDescription`] if `description` is empty
    /// after trimming; [`AgentDefinitionError::EmptyPrompt`] if `prompt` is.
    pub fn new(
        description: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<Self, AgentDefinitionError> {
        let description = description.into();
        let description = description.trim();
        if description.is_empty() {
            return Err(AgentDefinitionError::EmptyDescription);
        }
        let prompt = prompt.into();
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err(AgentDefinitionError::EmptyPrompt);
        }
        Ok(Self {
            description: description.to_string(),
            prompt: prompt.to_string(),
            tools: None,
            disallowed_tools: Vec::new(),
            model: None,
            max_turns: None,
            permission_mode: None,
            skills: Vec::new(),
            memory: None,
            mcp_servers: Vec::new(),
            initial_prompt: None,
            background: None,
            effort: None,
        })
    }

    /// Restrict the agent to exactly these tool names (omit to inherit all).
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Remove these tool names from the effective set (MCP patterns honored).
    #[must_use]
    pub fn with_disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.disallowed_tools = tools;
        self
    }

    /// Override the model for this agent (the per-subtask downgrade lever).
    #[must_use]
    pub fn with_model(mut self, model: ModelId) -> Self {
        self.model = Some(model);
        self
    }

    /// Cap this agent's own turn loop.
    #[must_use]
    pub const fn with_max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// Set the permission mode for tool calls inside this agent.
    #[must_use]
    pub const fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = Some(mode);
        self
    }

    /// Preload these skill names into the agent's context.
    #[must_use]
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }

    /// Set the memory scope this agent reads from.
    #[must_use]
    pub const fn with_memory(mut self, memory: MemorySource) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Attach external MCP servers scoped to this agent.
    #[must_use]
    pub fn with_mcp_servers(mut self, servers: Vec<McpServerConfig>) -> Self {
        self.mcp_servers = servers;
        self
    }

    /// Auto-submit this text as the first user turn when run as main thread.
    #[must_use]
    pub fn with_initial_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.initial_prompt = Some(prompt.into());
        self
    }

    /// Run this agent as a non-blocking background task when invoked.
    #[must_use]
    pub const fn with_background(mut self, background: bool) -> Self {
        self.background = Some(background);
        self
    }

    /// Override the reasoning-effort level for this agent.
    #[must_use]
    pub const fn with_effort(mut self, effort: EffortLevel) -> Self {
        self.effort = Some(effort);
        self
    }

    /// The natural-language description of when to use this agent.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The agent's own system prompt.
    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// The tool allowlist, if restricted; `None` inherits the parent's tools.
    #[must_use]
    pub fn tools(&self) -> Option<&[String]> {
        self.tools.as_deref()
    }

    /// Tool names removed from the effective set.
    #[must_use]
    pub fn disallowed_tools(&self) -> &[String] {
        &self.disallowed_tools
    }

    /// The model override; `None` inherits the parent model.
    #[must_use]
    pub const fn model(&self) -> Option<&ModelId> {
        self.model.as_ref()
    }

    /// The turn cap; `None` inherits the parent cap.
    #[must_use]
    pub const fn max_turns(&self) -> Option<u32> {
        self.max_turns
    }

    /// The permission mode; `None` inherits the parent mode.
    #[must_use]
    pub const fn permission_mode(&self) -> Option<PermissionMode> {
        self.permission_mode
    }

    /// Skill names preloaded into the agent's context; empty inherits none.
    #[must_use]
    pub fn skills(&self) -> &[String] {
        &self.skills
    }

    /// The memory scope; `None` inherits the parent's.
    #[must_use]
    pub const fn memory(&self) -> Option<MemorySource> {
        self.memory
    }

    /// External MCP servers scoped to this agent; empty inherits the parent's.
    #[must_use]
    pub fn mcp_servers(&self) -> &[McpServerConfig] {
        &self.mcp_servers
    }

    /// The auto-submitted first user turn when run as main thread; `None` if unset.
    #[must_use]
    pub fn initial_prompt(&self) -> Option<&str> {
        self.initial_prompt.as_deref()
    }

    /// Whether the agent runs as a non-blocking background task; `None` inherits.
    #[must_use]
    pub const fn background(&self) -> Option<bool> {
        self.background
    }

    /// The reasoning-effort override; `None` inherits the parent's.
    #[must_use]
    pub const fn effort(&self) -> Option<EffortLevel> {
        self.effort
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{AgentDefinition, AgentDefinitionError};
    use crate::agent::permissions::PermissionMode;
    use crate::types::ModelId;

    #[test]
    fn new_requires_non_empty_description_and_prompt() {
        assert_eq!(
            AgentDefinition::new("   ", "prompt"),
            Err(AgentDefinitionError::EmptyDescription)
        );
        assert_eq!(
            AgentDefinition::new("desc", "  "),
            Err(AgentDefinitionError::EmptyPrompt)
        );
    }

    #[test]
    fn new_trims_and_exposes_required_fields() {
        let def = AgentDefinition::new("  reviewer  ", "  be careful  ").expect("valid");
        assert_eq!(def.description(), "reviewer");
        assert_eq!(def.prompt(), "be careful");
        // Optional fields default to inherit-from-parent.
        assert_eq!(def.tools(), None);
        assert!(def.disallowed_tools().is_empty());
        assert_eq!(def.model(), None);
        assert_eq!(def.max_turns(), None);
        assert_eq!(def.permission_mode(), None);
    }

    #[test]
    fn with_setters_populate_optional_fields() {
        let def = AgentDefinition::new("desc", "prompt")
            .expect("valid")
            .with_tools(vec!["Read".to_string(), "Grep".to_string()])
            .with_disallowed_tools(vec!["Bash".to_string()])
            .with_model(ModelId::claude_haiku_4_5())
            .with_max_turns(3)
            .with_permission_mode(PermissionMode::DontAsk);
        assert_eq!(
            def.tools(),
            Some(["Read".to_string(), "Grep".to_string()].as_slice())
        );
        assert_eq!(def.disallowed_tools(), ["Bash".to_string()].as_slice());
        assert_eq!(def.model(), Some(&ModelId::claude_haiku_4_5()));
        assert_eq!(def.max_turns(), Some(3));
        assert_eq!(def.permission_mode(), Some(PermissionMode::DontAsk));
    }

    #[test]
    fn serializes_to_camelcase_wire_shape_omitting_inherits() {
        let def = AgentDefinition::new("reviewer", "be careful")
            .expect("valid")
            .with_tools(vec!["Read".to_string()])
            .with_disallowed_tools(vec!["Bash".to_string()])
            .with_model(ModelId::claude_haiku_4_5())
            .with_max_turns(2)
            .with_permission_mode(PermissionMode::DontAsk);
        let json = serde_json::to_value(&def).expect("serialize");
        assert_eq!(json["description"], "reviewer");
        assert_eq!(json["prompt"], "be careful");
        assert_eq!(json["tools"], serde_json::json!(["Read"]));
        assert_eq!(json["disallowedTools"], serde_json::json!(["Bash"]));
        assert_eq!(json["model"], "claude-haiku-4-5");
        assert_eq!(json["maxTurns"], 2);
        assert_eq!(json["permissionMode"], "dontAsk");
    }

    #[test]
    fn serialize_omits_unset_optional_fields() {
        let def = AgentDefinition::new("reviewer", "be careful").expect("valid");
        let json = serde_json::to_value(&def).expect("serialize");
        let obj = json.as_object().expect("object");
        assert!(!obj.contains_key("tools"));
        assert!(!obj.contains_key("disallowedTools"));
        assert!(!obj.contains_key("model"));
        assert!(!obj.contains_key("maxTurns"));
        assert!(!obj.contains_key("permissionMode"));
    }

    #[test]
    fn new_defaults_new_optional_fields_to_inherit() {
        let def = AgentDefinition::new("reviewer", "be careful").expect("valid");
        assert!(def.skills().is_empty());
        assert_eq!(def.memory(), None);
        assert!(def.mcp_servers().is_empty());
        assert_eq!(def.initial_prompt(), None);
        assert_eq!(def.background(), None);
        assert_eq!(def.effort(), None);
    }

    #[test]
    fn serialize_omits_unset_new_optional_fields() {
        let def = AgentDefinition::new("reviewer", "be careful").expect("valid");
        let json = serde_json::to_value(&def).expect("serialize");
        let obj = json.as_object().expect("object");
        assert!(!obj.contains_key("skills"));
        assert!(!obj.contains_key("memory"));
        assert!(!obj.contains_key("mcpServers"));
        assert!(!obj.contains_key("initialPrompt"));
        assert!(!obj.contains_key("background"));
        assert!(!obj.contains_key("effort"));
    }

    #[test]
    fn with_setters_populate_new_optional_fields() {
        use crate::agent::subagents::MemorySource;
        use crate::agent::types::EffortLevel;

        let def = AgentDefinition::new("reviewer", "be careful")
            .expect("valid")
            .with_skills(vec!["research".to_string()])
            .with_memory(MemorySource::Project)
            .with_initial_prompt("start here")
            .with_background(true)
            .with_effort(EffortLevel::High);
        assert_eq!(def.skills(), ["research".to_string()].as_slice());
        assert_eq!(def.memory(), Some(MemorySource::Project));
        assert_eq!(def.initial_prompt(), Some("start here"));
        assert_eq!(def.background(), Some(true));
        assert_eq!(def.effort(), Some(EffortLevel::High));
    }

    #[test]
    fn serializes_new_fields_to_camelcase_wire() {
        use crate::agent::subagents::MemorySource;
        use crate::agent::types::EffortLevel;

        let def = AgentDefinition::new("reviewer", "be careful")
            .expect("valid")
            .with_skills(vec!["research".to_string()])
            .with_memory(MemorySource::Project)
            .with_initial_prompt("start here")
            .with_background(true)
            .with_effort(EffortLevel::High);
        let json = serde_json::to_value(&def).expect("serialize");
        assert_eq!(json["skills"], serde_json::json!(["research"]));
        assert_eq!(json["memory"], "project");
        assert_eq!(json["initialPrompt"], "start here");
        assert_eq!(json["background"], true);
        assert_eq!(json["effort"], "high");
    }

    // `AgentMcpServerSpec` is undocumented upstream; this pins the ASSUMED
    // `[{name, config}]` element shape (reused `McpServerConfig`), not verified
    // parity. A future upstream correction should update this test, not be read
    // as a regression.
    #[test]
    fn serializes_mcp_servers_to_assumed_shape() {
        use crate::agent::types::McpServerConfig;

        let cfg = McpServerConfig::new("fs", serde_json::json!({"command": "node"}));
        let def = AgentDefinition::new("reviewer", "be careful")
            .expect("valid")
            .with_mcp_servers(vec![cfg]);
        let json = serde_json::to_value(&def).expect("serialize");
        assert_eq!(
            json["mcpServers"],
            serde_json::json!([{"name": "fs", "config": {"command": "node"}}])
        );
    }
}
