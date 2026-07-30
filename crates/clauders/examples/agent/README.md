# Agent SDK examples

Runnable examples for the `clauders` Agent SDK — the Rust client that drives the
`claude` Code CLI as a subprocess over its control protocol. Each example lives
in its own directory with a `main.rs` and a `README.md` that walks through the
SDK calls it uses.

## Prerequisites

- A `claude` binary version 2.0.0 or newer on `PATH` (or at
  `$HOME/.claude/local/claude`, or named explicitly with
  `Options::path_to_executable`).
- That binary already able to talk to Anthropic. **There is no `ANTHROPIC_API_KEY`
  in these examples** — the Agent SDK does not make HTTP requests itself, so it
  reuses whatever credentials the binary has. If `claude -p "hi"` works in your
  shell, these examples work.

## Run any example

Every example is registered by name in `crates/clauders/Cargo.toml`, so run it by
name from anywhere in the workspace. Agent example names carry an `agent_` prefix
so they never collide with the Messages API examples:

```text
cargo run -p clauders --example agent_01_query
```

The first run compiles the crate; reruns are fast.

## The examples

| # | Name | Shows |
|---|------|-------|
| 01 | `agent_01_query` | One-shot `query`: send a prompt, stream the frames |
| 02 | `agent_02_options` | The basic `Options` surface: prompt, model, tool gating, cwd, timeouts |
| 03 | `agent_03_session` | A stateful `Client`: several turns, handshake data, usage |
| 04 | `agent_04_partial_streaming` | Token-level `StreamEvent` frames, child stderr, unmodelled frames |
| 05 | `agent_05_permissions` | A `PermissionPolicy` that allows, rewrites arguments, and denies |
| 06 | `agent_06_hooks` | Hooks around the loop, including a blocking veto |
| 07 | `agent_07_mcp_tools` | In-process MCP tools written in Rust, plus MCP elicitation |
| 08 | `agent_08_structured_output` | Constrain the final answer to a JSON Schema; cap the spend |
| 09 | `agent_09_startup_options` | Thinking, effort, settings sources, skills, plugins, sandbox, betas |
| 10 | `agent_10_subagents` | Programmatic subagents the main agent delegates to |
| 11 | `agent_11_streaming_input` | Warm start, and feeding user turns in as a stream |
| 12 | `agent_12_live_control` | Mid-session control: interrupt, set model, MCP, files, tasks |
| 13 | `agent_13_sessions` | Resume and fork a session; read the on-disk transcript store |
| 14 | `agent_14_agent_console` | Interactive `ratatui` console: live tools, UI permission prompts, key-press interrupt |

## The shape shared by every example

```rust
use clauders::agent::{Message, Options, query};
use futures_util::StreamExt;

let options = Options::builder()
    .system_prompt("Answer in one line.")
    .max_turns(4)
    .build();

let mut stream = query("Say hi.", options).await?;
while let Some(frame) = stream.next().await {
    match frame? {
        Message::Assistant(assistant) => { /* model output */ }
        Message::Result(result)       => { /* the turn ended */ }
        _ => {}
    }
}
```

Three things generalize from that:

- **`Options` is the only configuration argument.** The same value configures a
  one-shot `query` and a long-lived `Client::connect`. Everything on it is fixed
  at spawn time; the mid-session equivalents live on `Client` (example 12).
- **The stream is a stream of frames, not of text.** `Message` is an exhaustive
  enum, so the compiler will not let a frame be dropped silently. `Message::Other`
  catches any frame kind this release does not model, so a newer binary cannot
  fail a turn.
- **Every turn ends with exactly one `Message::Result`.** It carries the session
  id, the turn count, cost, token usage, the structured output if one was
  requested, and why the turn stopped.

## `query` versus `Client`

`query(prompt, options)` is sugar for connect, send, and tear down: the returned
stream owns the session, which ends when the stream is dropped. Reach for
`Client::connect(options)` when you need more than one turn on the same history,
or any of the control operations — `interrupt`, `set_model`, `mcp_status`,
`get_usage` — which only exist while a client is alive.

## Handlers run in your process

Four traits let Rust code participate in the agent's loop rather than just observe
it. All four are registered on `Options` and consulted by the runtime as the turn
proceeds:

| Trait | Consulted when | Example |
|---|---|---|
| `PermissionPolicy` | a gated tool is about to run | 05, 14 |
| `Hook` | a lifecycle event fires (tool use, prompt, stop, …) | 06, 14 |
| `Tool` (via `SdkMcpServer`) | the model calls an in-process tool | 07 |
| `ElicitationPolicy` | an MCP server asks the user for input | 07 |

Each receives a `CancelSignal`. Cancellation is **cooperative**: if the binary
withdraws a request, nothing kills the handler task. A handler that ignores the
signal runs to completion and its answer is still delivered; one that cares should
check it and return early.
