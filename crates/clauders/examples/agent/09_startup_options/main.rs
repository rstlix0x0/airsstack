//! The rest of the startup surface: thinking, effort, settings sources,
//! skills, plugins, sandbox, betas, fallback model, and session identity.
//!
//! These all lower to flags on the `claude` process at spawn time, so none of
//! them can change once the session is running (example 12 covers the ones that
//! *do* have a mid-session equivalent).
//!
//! Two combinations are worth knowing about up front:
//!
//! - `thinking` wins over `max_thinking_tokens`; set one or the other.
//! - `sandbox` cannot be combined with `settings_path`. The sandbox config is
//!   merged into the settings payload, so a settings *file* would have to carry
//!   it instead. `settings_inline` is fine.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_09_startup_options
//! # optionally exercise beta flags and a local plugin directory:
//! CLAUDERS_AGENT_BETAS=some-beta CLAUDERS_AGENT_PLUGIN_DIR=./my-plugin \
//!   cargo run -p clauders --example agent_09_startup_options
//! ```

use clauders::agent::types::{
    PluginSpec, SandboxConfig, SessionPersistence, SettingSource, Skills, ThinkingConfig,
    ThinkingDisplay, ThinkingMode,
};
use clauders::agent::{ContentBlock, Message, Options, OptionsBuilder, PermissionMode, query};
use clauders::types::{EffortLevel, ModelId};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = Options::builder()
        .system_prompt("Answer in one short paragraph.")
        .permission_mode(PermissionMode::Plan)
        // Extended thinking. `Enabled { budget_tokens: Some(n) }` sets a hard
        // budget; `None` and `Adaptive` both let the binary decide.
        .thinking(ThinkingConfig {
            mode: ThinkingMode::Enabled {
                budget_tokens: Some(2048),
            },
            display: Some(ThinkingDisplay::Summarized),
        })
        // How hard the model works on each turn, independent of thinking.
        .effort(EffortLevel::Low)
        // Which settings files the binary loads. This also decides whether
        // on-disk agent and command definitions are visible.
        .setting_sources([SettingSource::User, SettingSource::Project])
        // Inline settings, rather than a path — required here because the
        // sandbox config below is merged into the same payload.
        .settings_inline(serde_json::json!({ "includeCoAuthoredBy": false }))
        // Sandboxing. `fail_if_unavailable: Some(false)` keeps the session
        // running on a machine with no sandbox support.
        .sandbox(SandboxConfig {
            enabled: Some(true),
            fail_if_unavailable: Some(false),
            ..SandboxConfig::default()
        })
        // Skills are gated through the allowed-tools list, not a flag of their
        // own: `All` permits the bare `Skill` tool, `List` permits named ones.
        .skills(Skills::All)
        // Only consult MCP servers this program declared; ignore project, user,
        // and plugin MCP config.
        .strict_mcp_config(true)
        // Used when the primary model is overloaded.
        .fallback_model(ModelId::claude_haiku_4_5())
        // Session identity. A title is sent in the handshake; disabling
        // persistence makes the session unresumable.
        .title("startup options tour")
        .session_persistence(SessionPersistence::Disabled)
        .max_turns(2);

    // Beta flags and plugin directories are environment-specific, so they are
    // only added when the caller names them.
    builder = with_optional_betas(builder);
    builder = with_optional_plugin(builder);

    let options = builder.build();
    println!("{options:#?}");
    println!("---");

    let mut stream = query("Why does Rust have both String and &str?", options).await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => {
                for block in &assistant.content {
                    match block {
                        // Present because thinking is enabled and its display
                        // is `Summarized`; `Omitted` would suppress these.
                        ContentBlock::Thinking { thinking } => {
                            println!("[thinking] {thinking}");
                        }
                        ContentBlock::Text { text } => println!("{text}"),
                        _ => {}
                    }
                }
            }
            Message::Result(result) => {
                println!("---");
                for (model, usage) in &result.model_usage {
                    println!("{model}: {usage:?}");
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Add `--betas` entries from `CLAUDERS_AGENT_BETAS` (comma-separated).
fn with_optional_betas(builder: OptionsBuilder) -> OptionsBuilder {
    match std::env::var("CLAUDERS_AGENT_BETAS") {
        Ok(raw) if !raw.trim().is_empty() => builder.betas(
            raw.split(',')
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty()),
        ),
        _ => builder,
    }
}

/// Load a local plugin directory named by `CLAUDERS_AGENT_PLUGIN_DIR`.
fn with_optional_plugin(builder: OptionsBuilder) -> OptionsBuilder {
    match std::env::var("CLAUDERS_AGENT_PLUGIN_DIR") {
        Ok(dir) if std::path::Path::new(&dir).is_dir() => builder.plugin(PluginSpec {
            path: dir.into(),
            // `true` selects `--plugin-dir-no-mcp`: load the plugin without
            // discovering the MCP servers it declares.
            skip_mcp_discovery: false,
        }),
        _ => builder,
    }
}
