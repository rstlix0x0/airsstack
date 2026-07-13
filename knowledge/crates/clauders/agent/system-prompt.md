---
type: Rust Module
title: clauders::agent::system_prompt
description: SystemPromptConfig — user-facing system-prompt configuration on Options (None/Text/Preset), distinct from the wire-level SystemPrompt; each runtime lowers it to its own representation at request-build time.
tags: [rust, sdk, agent, system-prompt, configuration]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/system_prompt.rs
---

Newly landed alongside the runtime-adapter regroup. Carried on
[`Options`](/crates/clauders/agent/options.md) (field `system_prompt`,
now typed `SystemPromptConfig` rather than a bare `Option<String>` —
that field's type changed under this same landing, though `agent/options.rs`
itself is outside this concept's source scope). Explicitly distinct from
the wire-level [`crate::types::SystemPrompt`](/crates/clauders/types/system.md):
this type expresses *caller intent* (including the `claude_code` preset,
which has no wire representation of its own — it is a CLI-resident base
prompt, not something sent over the wire), while `types::SystemPrompt` is
the Messages-API request shape. Each runtime lowers a `SystemPromptConfig`
to its own representation at request-build time.

# Schema

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SystemPromptConfig {
    #[default]
    None,
    Text(String),
    Preset {
        append: Option<String>,
        exclude_dynamic_sections: bool,
    },
}

impl SystemPromptConfig {
    pub fn native_text(&self) -> Option<String>;
    pub const fn is_preset(&self) -> bool;
}

impl From<String> for SystemPromptConfig { .. } // -> Text
impl From<&str> for SystemPromptConfig { .. }   // -> Text
```

`Preset` is the built-in `claude_code` base prompt, optionally extended:
`append` is extra instructions appended after the base;
`exclude_dynamic_sections` moves per-session dynamic sections (cwd, git
status, …) out of the system prompt and into the first user message
instead.

`native_text()` — the plain text a *native* (non-CLI) runtime should send
as its system prompt: `None` → `None`; `Text(s)` → `Some(s)`; `Preset {
append, .. }` → `append.clone()` (the `claude_code` base itself is
dropped — it is unavailable off the CLI binary, so a preset degrades to
just its append text; a base-less preset with `append: None` yields
`None`, i.e. no system prompt at all).

`is_preset()` — `true` only for the `Preset` variant; used by native
runtimes purely to decide whether to log a degrade warning (`native_text`
already computes the correct degraded value regardless).

# Per-runtime lowering

| Runtime | How it lowers `SystemPromptConfig` |
| --- | --- |
| [CliRuntime](/crates/clauders/agent/cli/runtime.md) | Matches the enum directly in [`argv::build_argv`](/crates/clauders/agent/cli/argv.md): `Preset` → `--append-system-prompt <append>` plus, if set, `--exclude-dynamic-system-prompt-sections` (the real binary flag); `Text` → `--system-prompt <text>`. It also sends `native_text()` as the handshake's `system_prompt` field in [`handshake::initialize_request`](/crates/clauders/agent/cli/handshake.md) — so the preset's `claude_code` base is requested via the argv flag, never re-sent as literal text. |
| [ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) | Calls `is_preset()` to `tracing::warn!` that the preset base is unavailable, then always uses `native_text()` (mapped to a wire [`SystemPrompt::text`](/crates/clauders/types/system.md)) — a preset DEGRADES to its `append` alone; no `claude_code` base is ever sent (there is no binary to own it). |
| [OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md) | Same degrade pattern as `ApiRuntime`: warns on `is_preset()`, then uses `native_text()` as a plain system-message string. |

# Examples

```rust
use clauders::agent::SystemPromptConfig;

assert_eq!(SystemPromptConfig::default(), SystemPromptConfig::None);
assert_eq!(SystemPromptConfig::from("be terse").native_text(), Some("be terse".to_owned()));

let preset = SystemPromptConfig::Preset { append: Some("extra".to_owned()), exclude_dynamic_sections: false };
assert_eq!(preset.native_text(), Some("extra".to_owned()));
assert!(preset.is_preset());
```

Related: [Options](/crates/clauders/agent/options.md),
[types::SystemPrompt (wire)](/crates/clauders/types/system.md),
[cli::argv::build_argv](/crates/clauders/agent/cli/argv.md),
[cli::handshake::initialize_request](/crates/clauders/agent/cli/handshake.md),
[ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md),
[OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md).

# Citations

1. `crates/clauders/src/agent/system_prompt.rs`
