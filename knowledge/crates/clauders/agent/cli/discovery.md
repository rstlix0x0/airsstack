---
type: Rust Module
title: clauders::agent::cli::discovery
description: discover / check_version — locates the claude binary (override, PATH, per-user fallback) and gates its reported version against the SDK's supported minimum.
tags: [rust, sdk, agent, cli, discovery, versioning]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/agent/cli/discovery.rs
---

# Schema

```rust
const MIN_VERSION: (u32, u32, u32) = (2, 0, 0);
const BINARY_NAME: &str = "claude";

pub(super) fn discover(options: &Options) -> Result<PathBuf, AgentError>;
fn discover_in(options: &Options, path_dirs: &[PathBuf], home: Option<&Path>) -> Result<PathBuf, AgentError>;
pub(super) fn check_version(found: &str, require_min: bool) -> Result<(), AgentError>;
fn parse_semver(s: &str) -> Option<(u32, u32, u32)>;
```

Resolution order: `Options::path_to_executable` override (must exist, no
further search), then each `PATH` directory, then
`$HOME/.claude/local/claude`. Failure returns `AgentError::BinaryNotFound`
carrying every path inspected. `discover_in` is the environment-injected
core, kept separate for deterministic testing.

`check_version`: below `MIN_VERSION` and `require_min == false` → a
`tracing::warn!`, request proceeds; below minimum and `require_min == true`
→ hard `AgentError::BinaryVersionUnsupported`; unparseable version string →
warn and allow (forward-compat).

# Examples

```rust
# fn example() {
// check_version("1.5.0", false) -> Ok(()) with a warning logged
// check_version("1.5.0", true)  -> Err(AgentError::BinaryVersionUnsupported { .. })
# }
```

Related: [Options::path_to_executable/require_min_version](/crates/clauders/agent/options.md),
[AgentError::BinaryNotFound/BinaryVersionUnsupported](/crates/clauders/agent/error.md),
[CliRuntime::connect](/crates/clauders/agent/cli/runtime.md) (calls `discover`
then probes `--version` and calls `check_version`).

# Citations

1. `crates/clauders/src/agent/cli/discovery.rs`
