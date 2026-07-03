---
type: Rust Module
title: clauders::agent::cli::argv
description: build_argv — maps session Options to the claude binary's full argument vector; permission_mode_wire renders a PermissionMode as the binary's camelCase wire spelling.
tags: [rust, sdk, agent, cli, argv]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/cli/argv.rs
---

# Schema

```rust
pub(super) fn build_argv(options: &Options) -> Vec<String>;
pub(super) const fn permission_mode_wire(mode: PermissionMode) -> &'static str;
```

Order: caller-supplied `executable_args` first, then SDK-managed
stream-protocol flags (`--output-format stream-json --input-format
stream-json --verbose`), then `--permission-mode <wire>`, then optional
mapped fields (`--permission-prompt-tool stdio` when a
`permission_policy` is registered, `--model`, `--system-prompt`,
`--allowed-tools`, `--disallowed-tools`, `--max-turns`, one
`--mcp-config <json>` per configured MCP server). `cwd` and `env` are not
argv — applied to the process spawn config instead.

# Examples

```rust,no_run
# fn example() {
use clauders::agent::Options;
// build_argv(&Options::default()) always includes:
// "--output-format" "stream-json" "--input-format" "stream-json" "--verbose"
// "--permission-mode" "default"
# }
```

Related: [Options](/crates/clauders/agent/options.md),
[PermissionMode](/crates/clauders/agent/permissions.md),
[ProcessConfig::args](/crates/clauders/agent/process/spawn.md),
[CliRuntime::connect](/crates/clauders/agent/cli/runtime.md) (calls `build_argv`).

# Citations

1. `crates/clauders/src/agent/cli/argv.rs`
