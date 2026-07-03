---
type: Rust Module
title: clauders::agent::options
description: Options / OptionsBuilder — session configuration for a Client/query call, carrying everything the runtime needs to discover, spawn, and configure the backend binary.
tags: [rust, sdk, agent, configuration, builder]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/options.rs
---

Built via `Options::builder()`. In-loop handler fields (`hooks`,
`permission_policy`) carry `Arc`-wrapped handlers consulted by the
runtime's background reader.

# Schema

```rust
pub struct Options {
    pub system_prompt: Option<String>,
    pub model: Option<ModelId>,
    pub permission_mode: PermissionMode,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub mcp_servers: Vec<McpServerConfig>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub max_turns: Option<u32>,
    pub path_to_executable: Option<PathBuf>,
    pub executable_args: Vec<String>,
    pub require_min_version: bool,
    pub shutdown_grace: Duration,       // default 5s
    pub hooks: HookRegistry,
    pub permission_policy: Option<Arc<dyn PermissionPolicy>>,
}
```

`Debug` is hand-written so `hooks`/`permission_policy` print only a
presence indicator, never handler internals.
`OptionsBuilder` mirrors every field with a setter; `hook(event, matcher, hook)`
registers into the embedded `HookRegistry`; `build()` applies the 5-second
default `shutdown_grace` when unset.

# Examples

```rust
use clauders::agent::{Options, PermissionMode};
let opts = Options::builder()
    .permission_mode(PermissionMode::AcceptEdits)
    .allowed_tools(vec!["Bash".to_string()])
    .max_turns(7)
    .build();
assert_eq!(opts.permission_mode, PermissionMode::AcceptEdits);
```

Related: [PermissionMode / PermissionPolicy](/crates/clauders/agent/permissions.md),
[Hook / HookRegistry](/crates/clauders/agent/hooks.md),
[McpServerConfig](/crates/clauders/agent/types/mcp.md),
[cli::argv::build_argv](/crates/clauders/agent/cli/argv.md) (maps `Options`
to the backend's argument vector), [CliRuntime::connect](/crates/clauders/agent/cli/runtime.md).

# Citations

1. `crates/clauders/src/agent/options.rs`
