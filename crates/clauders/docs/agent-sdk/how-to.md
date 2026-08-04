# How-to guides: Agent SDK

Recipes indexed by what you are trying to do. Each one names a runnable example — every example is a
complete program with its own `README.md` walking through the calls it makes.

If you have not used the crate before, start with the [tutorial](tutorial.md) instead.

## Prerequisites for every recipe

A `claude` binary 2.0.0 or newer on `PATH`, or at `$HOME/.claude/local/claude`, or named explicitly
with `Options::path_to_executable`.

No `ANTHROPIC_API_KEY`. The Agent SDK drives the binary rather than calling the API, so it reuses
whatever credentials that binary holds. If `claude -p "hi"` works, the examples work.

Examples are registered by name in `Cargo.toml` and run from anywhere in the workspace. Agent example
names carry an `agent_` prefix so they never collide with the Messages API examples.

```bash
cargo run -p clauders --example agent_01_query
```

---

## Getting output

### Send one prompt and read the reply

```bash
cargo run -p clauders --example agent_01_query
```

`query(prompt, Options)` → a `MessageStream` that owns the session. Match `Message::Assistant` for
text and `Message::Result` for the terminal frame.
→ [`examples/agent/01_query/`](../../examples/agent/01_query/README.md)

### Stream tokens as they are produced, not in whole blocks

```bash
cargo run -p clauders --example agent_04_partial_streaming
```

`Options::include_partial_messages` turns on `Message::StreamEvent` frames. The same example shows
capturing the child's stderr and what an unmodelled frame looks like as `Message::Other`.
→ [`examples/agent/04_partial_streaming/`](../../examples/agent/04_partial_streaming/README.md)

### Force the final answer into a JSON Schema

```bash
cargo run -p clauders --example agent_08_structured_output
```

`Options::output_schema` or `output_format`; the parsed value arrives on
`ResultMessage::structured_output`. The example also caps spend with `max_budget_usd`.
→ [`examples/agent/08_structured_output/`](../../examples/agent/08_structured_output/README.md)

---

## Controlling the agent

### Set the model, gate the tools, choose a working directory

```bash
cargo run -p clauders --example agent_02_options
```

The core of `Options`: `system_prompt`, `model`, `allowed_tools`, `disallowed_tools`, `cwd`,
`max_turns`, and the timeout knobs.
→ [`examples/agent/02_options/`](../../examples/agent/02_options/README.md)

### Configure thinking, effort, skills, plugins, sandbox, or betas

```bash
cargo run -p clauders --example agent_09_startup_options
```

The startup options that lower to CLI flags — `thinking`, `effort`, `setting_sources`, `skills`,
`plugins`, `sandbox`, `betas`. All fixed at spawn time.
→ [`examples/agent/09_startup_options/`](../../examples/agent/09_startup_options/README.md)

### Delegate sub-tasks to purpose-built subagents

```bash
cargo run -p clauders --example agent_10_subagents
```

`Options::agent(name, AgentDefinition)` builds the map lowered to the binary's `--agents` JSON. Each
definition carries its own prompt, tools, model, and permission mode.
→ [`examples/agent/10_subagents/`](../../examples/agent/10_subagents/README.md)

### Change the model or permission mode without restarting

```bash
cargo run -p clauders --example agent_12_live_control
```

`Client::set_model`, `set_permission_mode`, `interrupt`, `mcp_status`, `read_file`, `stop_task`,
`get_usage`, `get_context_usage`. These exist only while a `Client` is alive.
→ [`examples/agent/12_live_control/`](../../examples/agent/12_live_control/README.md)

---

## Running your own code inside the loop

Four traits let Rust participate in the turn rather than observe it. All four are registered on
`Options` and called by the runtime as the turn proceeds. Each receives a `CancelSignal`, and
cancellation is cooperative — see [explanation.md](explanation.md).

### Approve, rewrite, or deny a tool call

```bash
cargo run -p clauders --example agent_05_permissions
```

Implement `PermissionPolicy`. Return `Allow { updated_input }` to rewrite the arguments before they
run, or `Deny { message, interrupt }` to refuse.
→ [`examples/agent/05_permissions/`](../../examples/agent/05_permissions/README.md)

### React to lifecycle events, or veto one

```bash
cargo run -p clauders --example agent_06_hooks
```

Implement `Hook` and register it against a `HookEvent` with an optional matcher. Returning a blocking
`HookOutput` stops the action.
→ [`examples/agent/06_hooks/`](../../examples/agent/06_hooks/README.md)

### Expose a Rust function as a tool the model can call

```bash
cargo run -p clauders --example agent_07_mcp_tools
```

`tool(name, description, schema, closure)` and `SdkMcpServer::builder(...)`. The server runs
in-process — no separate MCP server to launch. The same example implements an `ElicitationPolicy` for
when an MCP server asks the user for input mid-call.
→ [`examples/agent/07_mcp_tools/`](../../examples/agent/07_mcp_tools/README.md)

---

## Sessions

### Continue, resume, or fork a previous session

```bash
cargo run -p clauders --example agent_13_sessions
```

`SessionControl::{New, Continue { fork }, Resume { id, fork, resume_at }}` on `Options::session`,
lowering to `--continue`, `--resume <id>`, `--fork-session`, and `--resume-session-at=<uuid>`.
→ [`examples/agent/13_sessions/`](../../examples/agent/13_sessions/README.md)

### List past sessions or read a transcript off disk

Same example. `SessionArchive` reads the `.jsonl` transcripts the CLI itself writes, without going
through the subprocess at all:

| Method | Does |
|---|---|
| `SessionArchive::list(ListOptions)` | enumerate sessions for a project directory |
| `SessionArchive::info(id, …)` | metadata for one session |
| `SessionArchive::messages(id, MessagesOptions)` | reconstruct the conversation |
| `SessionArchive::rename(id, title)` | set a session's display title |
| `SessionArchive::tag(id, tag)` | tag a session |

### Cut the startup latency of the first turn

```bash
cargo run -p clauders --example agent_11_streaming_input
```

`Client::startup(options)` returns a `WarmQuery` — the subprocess is spawned and the handshake
completed before you have a prompt. Calling `.query(prompt)` on it yields a `WarmSession` with no
spawn delay. The same example feeds user turns in as a stream via `Prompt::Stream`.
→ [`examples/agent/11_streaming_input/`](../../examples/agent/11_streaming_input/README.md)

---

## Building something interactive

### Drive an agent from a terminal UI

```bash
cargo run -p clauders --example agent_14_agent_console
```

A `ratatui` console: live tool activity, permission prompts answered from the UI, and a key press
that interrupts the running turn. The most complete program in the set, and the one to read if you
are wiring the SDK into a real application.
→ [`examples/agent/14_agent_console/`](../../examples/agent/14_agent_console/README.md)

---

## Choosing between `query` and `Client`

`query(prompt, options)` is connect, send, and tear down in one call — the returned stream owns the
session and ends it when dropped. Use it for one-shot work.

`Client::connect(options)` keeps the subprocess alive. Reach for it when you need more than one turn
on the same history, or any control operation — `interrupt`, `set_model`, `mcp_status`, `get_usage`
— since those exist only while a client is alive.
