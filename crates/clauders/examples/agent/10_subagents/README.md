# 10 — Subagents

Named helpers, defined in Rust, that the main agent can delegate a subtask to.

## Run

```text
cargo run -p clauders --example agent_10_subagents
```

## What it shows

```rust
let summarizer = AgentDefinition::new(
    "Summarizes a file in three bullet points. Use for any 'summarize this file' request.",
    "You summarize source files. Read the file, then reply with exactly three bullet points.",
)?
.with_tools(vec!["Read".to_owned(), "Glob".to_owned()])
.with_model(ModelId::claude_haiku_4_5())
.with_effort(EffortLevel::Low)
.with_max_turns(3)
.with_permission_mode(PermissionMode::Plan);

let options = Options::builder()
    .agent("summarizer", summarizer)
    .agent("counter", counter)
    .allowed_tools(vec!["Task".to_owned(), "Bash".to_owned(), "Glob".to_owned()])
    .build();
```

`AgentDefinition::new(description, prompt)` validates both required fields non-empty
at construction — an empty one is an `AgentDefinitionError` where you wrote it, not a
confusing failure later. The registration name is what the model invokes; registering
the same name twice replaces the earlier definition.

The main agent needs the `Task` tool in its allowlist to delegate at all.

## The two roles of the two strings

- **`description`** is read by the *model* when choosing a helper. Write it as
  selection criteria: what this agent is for and when to pick it.
- **`prompt`** is that helper's own system prompt, in force only inside its subtask.

## Override or inherit

Every optional field overrides the parent session when set and inherits it when not.
That is what makes a subagent the natural place to run a narrow task on a cheaper
model with fewer tools:

| Method | Unset means |
|---|---|
| `with_tools(vec![…])` | inherit every tool the parent has |
| `with_disallowed_tools(vec![…])` | subtract nothing |
| `with_model(id)` | inherit the parent model |
| `with_max_turns(n)` | inherit the parent cap |
| `with_permission_mode(mode)` | inherit the parent mode |
| `with_effort(level)` | inherit the parent effort |
| `with_skills(vec![…])` | preload no skills |
| `with_memory(MemorySource::…)` | inherit the parent's memory scope |
| `with_mcp_servers(vec![…])` | inherit the parent's servers |
| `with_initial_prompt(text)` | no auto-submitted first turn |
| `with_background(true)` | run in the foreground |

Every field also has a getter, so a definition can be inspected after it is built:

```rust
definition.model().map(ModelId::as_str);
definition.tools();
definition.max_turns();
```

`MemorySource` is `User`, `Project`, or `Local`.

**One caveat on `with_mcp_servers`:** the official element shape for per-agent MCP
servers is undocumented, so this serializes each server as `{ "name": …, "config": … }`
— the official form likely inlines transport fields instead. Verify against a live
round-trip before relying on it.

## Attributing delegated work

```rust
let origin = assistant.parent_tool_use_id
    .as_deref()
    .map_or_else(|| "main".to_owned(), |id| format!("subagent {id}"));
```

A turn produced *by* a subagent carries the id of the tool call that spawned it, so
delegated output stays attributable in a mixed stream. Frames with
`parent_tool_use_id: None` came from the main thread.

## Delegating versus becoming

`Options::agents` registers helpers the running agent may delegate to.
`Options::agent_name("reviewer")` is the different lever: it selects **one** agent to
run as the session itself, instead of the default assistant. On-disk agent definitions
are loaded through `setting_sources` (example 09), not through this map.
