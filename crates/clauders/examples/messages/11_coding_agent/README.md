# 11 — Agentic coding CLI (terminal UI)

A small command-line coding agent with a **terminal UI**, built on the Messages
SDK. It is `07_agentic_tool_loop` with two things added: the tools are **real**
(the model writes files, reads them back, and runs `cargo`), and the run is drawn
in a live [ratatui](https://ratatui.rs) interface instead of scrolling `println!`s.

The model can scaffold a Rust program, compile it, read the compiler errors, fix
its own code, and repeat — until the program builds and runs. You watch it happen.

## Run

```text
# default task: build and run a FizzBuzz crate
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 11_coding_agent

# your own task, after `--`
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 11_coding_agent -- \
  "write a Rust program that prints the first 20 primes, then run it"
```

Keys: `q`/`Esc` quit · `↑`/`↓` and `PageUp`/`PageDown` scroll · `End` re-follow the
newest line.

Everything the agent does happens inside a **sandbox directory**
(`./coding-agent-workspace`, created next to where you run the command). It is not
wiped between runs — delete it yourself to start clean.

## Structure

The UI must stay responsive while the agent waits on the network and on `cargo`,
so the two run apart and talk over a channel:

```
main ──┬─ tokio::spawn(run_agent …)  ── background: the SDK agent loop
       │        │  AgentEvent over an mpsc channel
       │        ▼
       └─ run_ui(…)                   ── foreground: draw + poll keys + drain events
```

- **`run_agent`** is the agent (below). Instead of printing, it sends
  `AgentEvent::{Status, Line, Done}` down the channel.
- **`run_ui`** owns the terminal. Each ~100 ms it redraws, polls for a keystroke,
  and drains whatever the agent has sent. Because `main` runs on the multi-threaded
  runtime, the agent task keeps making progress while the UI blocks on input polling.

## The SDK part

Identical to `07`. Define the three tools (same `Tool` shape as `03`/`07`):

```rust
Tool {
    name: ToolName::new("write_file")?,
    description: "Create or overwrite a text file inside the sandbox.".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "path":     { "type": "string" },
            "contents": { "type": "string" }
        },
        "required": ["path", "contents"]
    }),
    cache_control: None, strict: None, eager_input_streaming: None,
}
```

Steer the agent with a system prompt (`SystemPrompt::text`) and run the loop: send
the conversation, stop when `stop_reason != ToolUse`, otherwise run every tool call
and feed the results back —

```rust
let result = match run_tool(root, tu.name.as_str(), &tu.input).await {
    Ok(out)  => ToolResultBlock::text(tu.id.clone(), truncate(out)),
    Err(err) => ToolResultBlock::err(tu.id.clone(), err),   // model can read + recover
};
results.push(ContentBlockParam::ToolResult(result));
```

`ToolResultBlock::text` for success, `ToolResultBlock::err` (sets `is_error: true`)
for a failure the model should see and fix — e.g. a bad path or a cargo error.

## Safety

This runs a real subprocess (`cargo`) and writes real files, so it is fenced in:

- **Path sandboxing.** Every `path` is rejected if it is absolute or contains `..`,
  then joined onto the sandbox root — a tool call cannot read or write outside
  `./coding-agent-workspace`.
- **Pinned working directory.** `cargo` always runs with `current_dir` set to the
  sandbox.
- **Bounded cost.** The loop is capped at 20 turns and each tool result is truncated
  before being fed back.

Even so, this is example code: it runs `cargo` with whatever arguments the model
supplies. Run it where you are comfortable doing that, and watch the transcript.

## Notes

- `ratatui` is a dev-dependency (it re-exports `crossterm`, so no separate crossterm
  dep). The SDK itself needs none of this — the TUI is pure presentation over the
  same agent loop as `07`.
- The API key and client are built **before** entering raw mode, so a missing key
  prints a normal error instead of being swallowed by the alternate screen.
- To extend it into a general coding agent, add tools (`run_command`, `list_files`,
  `search`) the same way and describe them in the system prompt.
