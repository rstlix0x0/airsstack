---
type: Rust Module
title: clauders::agent::runtime::cli::argv
description: build_argv — maps session Options to the claude binary's full argument vector (including SystemPromptConfig lowering and SDK-MCP declarations); permission_mode_wire renders a PermissionMode as the binary's camelCase wire spelling.
tags: [rust, sdk, agent, cli, argv, system-prompt]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/cli/argv.rs
---

Relocated from `agent/cli/argv.rs` to `agent/runtime/cli/argv.rs` in the
runtime-adapter regroup (`agent::runtime::{cli,api,openrouter,routing}`);
same module, same `pub(super)` surface — see the
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Schema

```rust
pub(super) fn build_argv(options: &Options) -> Vec<String>;
pub(super) const fn permission_mode_wire(mode: PermissionMode) -> &'static str;
```

Order: caller-supplied `executable_args` first, then SDK-managed
stream-protocol flags (`--output-format stream-json --input-format
stream-json --verbose`), then `--permission-mode <wire>`, then optional
mapped fields (`--permission-prompt-tool stdio` when a
`permission_policy` is registered, `--model`, the
[`SystemPromptConfig`](/crates/clauders/agent/system-prompt.md) lowering
described below, `--allowed-tools`, `--disallowed-tools`, `--max-turns`,
one `--mcp-config <json>` per configured `McpServerConfig`, then one more
`--mcp-config` per in-process SDK MCP server declaration read from
`options.sdk_mcp_servers`). `cwd` and `env` are not argv — applied to the
process spawn config instead.

`options.system_prompt` is a
[`SystemPromptConfig`](/crates/clauders/agent/system-prompt.md), matched
directly — this is the one runtime that does NOT degrade a preset:

| Variant | argv emitted |
| --- | --- |
| `None` | nothing |
| `Text(text)` | `--system-prompt <text>` |
| `Preset { append: Some(a), .. }` | `--append-system-prompt <a>` (keeps the CLI's built-in `claude_code` base; never emits `--system-prompt`) |
| `Preset { append: None, .. }` | nothing from `append` |
| `Preset { exclude_dynamic_sections: true, .. }` | additionally `--exclude-dynamic-system-prompt-sections` |

`--exclude-dynamic-system-prompt-sections` is a real binary flag only
`CliRuntime` can honor faithfully: only the CLI has the `claude_code` base
prompt to append onto and dynamic sections (cwd, git status, …) to move.
The native [ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md) and
[OpenRouterRuntime](/crates/clauders/agent/runtime/openrouter/runtime.md)
have no such base and degrade a preset to its `append` text alone via
`SystemPromptConfig::native_text`.

# Examples

```rust,no_run
# fn example() {
use clauders::agent::Options;
// build_argv(&Options::default()) always includes:
// "--output-format" "stream-json" "--input-format" "stream-json" "--verbose"
// "--permission-mode" "default"

// A preset with append + exclude_dynamic_sections:
// Options::builder().system_prompt_preset(Some("extra rules".into()), true).build()
// -> ".. --append-system-prompt extra rules --exclude-dynamic-system-prompt-sections .."
# }
```

Related: [Options](/crates/clauders/agent/options.md),
[SystemPromptConfig](/crates/clauders/agent/system-prompt.md),
[PermissionMode](/crates/clauders/agent/permissions.md),
[ProcessConfig::args](/crates/clauders/agent/process/spawn.md),
[CliRuntime::connect](/crates/clauders/agent/cli/runtime.md) (calls `build_argv`),
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Citations

1. `crates/clauders/src/agent/runtime/cli/argv.rs`
2. `crates/clauders/src/agent/system_prompt.rs`
