# 09 — Startup options

The rest of the pre-spawn surface: thinking, effort, settings sources, skills,
plugins, sandbox, betas, fallback model, and session identity.

## Run

```text
cargo run -p clauders --example agent_09_startup_options

# optionally exercise beta flags and a local plugin directory:
CLAUDERS_AGENT_BETAS=some-beta CLAUDERS_AGENT_PLUGIN_DIR=./my-plugin \
  cargo run -p clauders --example agent_09_startup_options
```

Betas and plugin directories are environment-specific, so the example only sets them
when you name them.

## Two combinations to know

- **`thinking` wins over `max_thinking_tokens`.** The scalar field is used only when
  `thinking` is `None`. Set one or the other.
- **`sandbox` cannot be combined with `settings_path`.** The sandbox config is merged
  into the settings payload, so a settings *file* would have to carry it instead.
  Combining them fails at connect with `AgentError::SandboxWithSettingsPath`.
  `settings_inline` is fine, which is what this example uses.

## Thinking

```rust
.thinking(ThinkingConfig {
    mode: ThinkingMode::Enabled { budget_tokens: Some(2048) },
    display: Some(ThinkingDisplay::Summarized),
})
```

`ThinkingMode` is `Enabled { budget_tokens }`, `Disabled`, or `Adaptive`. A budget of
`None` under `Enabled` is treated as adaptive. `display` is `Summarized` or `Omitted`
and is ignored when the mode is `Disabled`.

With thinking on and display `Summarized`, turns carry `ContentBlock::Thinking`
blocks:

```rust
ContentBlock::Thinking { thinking } => println!("[thinking] {thinking}"),
```

`effort` is the separate lever — `Low` through `Max`, how hard the model works per
turn regardless of thinking. `Client::set_max_thinking_tokens` (example 12) is the
mid-session equivalent of the budget.

## Settings

Two similarly named fields doing different jobs:

| Field | Meaning |
|---|---|
| `setting_sources([SettingSource::User, ::Project, ::Local])` | *which* settings files the binary loads — and with them, which on-disk agent and command definitions are visible |
| `settings_path(path)` / `settings_inline(json)` | the value of the separate `--settings` flag: one settings document, as a file or inline |

The singular/plural split mirrors the binary's own `--setting-sources` versus
`--settings`.

## Sandbox

```rust
.sandbox(SandboxConfig {
    enabled: Some(true),
    fail_if_unavailable: Some(false),
    ..SandboxConfig::default()
})
```

Only `enabled` and `fail_if_unavailable` are typed; the rest of the sandbox schema
(network, filesystem, credentials, deny lists) passes through `SandboxConfig::extra`
untyped, because that schema drifts between binary versions.
`fail_if_unavailable: Some(false)` keeps the session running on a machine with no
sandbox support.

## Skills

```rust
.skills(Skills::All)              // permits the bare `Skill` tool
.skills(Skills::List(vec![...]))  // permits `Skill(<name>)` for each named skill
```

Skills are gated through the allowed-tools list, not a flag of their own.

## Plugins

```rust
.plugin(PluginSpec { path: dir.into(), skip_mcp_discovery: false })
```

Local plugin directories, appended one at a time. `skip_mcp_discovery: true` loads the
plugin without discovering the MCP servers it declares.

## Session identity

- `title("…")` — a display title sent in the handshake.
- `session_id(SessionId)` — forces a specific id for a *new* session. The binary
  requires a valid UUID and rejects anything else.
- `session_persistence(SessionPersistence::Disabled)` — makes the session ephemeral
  and therefore unresumable. The default is `Enabled`, which is what example 13 needs.

## The rest

- `fallback_model(ModelId)` — used when the primary model is overloaded. Per-model
  attribution shows up in `ResultMessage::model_usage`, keyed by model id.
- `strict_mcp_config(true)` — consult only the MCP servers this program declared.
- `betas([...])` — beta feature flags forwarded to the binary.
- `permission_prompt_tool_name(name)` — override the default permission bridge with a
  named MCP tool.
- `user(id)` — an opaque caller identifier, carried for API-shape parity. It has no
  effect on the CLI runtime, because the binary exposes no matching flag.
