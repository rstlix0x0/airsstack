# 02 — Options

The basic session configuration: what the agent is told, which model runs, which
tools it may use, where it runs, and how long the SDK waits on it.

## Run

```text
cargo run -p clauders --example agent_02_options
```

## What it shows

```rust
let options = Options::builder()
    .system_prompt("You are a terse code reader. Answer in at most three lines.")
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(2048))
    .permission_mode(PermissionMode::Plan)
    .allowed_tools(vec!["Read".to_owned(), "Grep".to_owned(), "Glob".to_owned()])
    .disallowed_tools(vec!["Bash".to_owned()])
    .cwd(&cwd)
    .add_dir(cwd.join("src"))
    .env("CLAUDERS_EXAMPLE", "agent_02_options")
    .max_turns(4)
    .control_request_timeout(Duration::from_secs(30))
    .shutdown_grace(Duration::from_secs(2))
    .require_min_version(true)
    .build();
```

`Options::builder()` is a plain builder — `build()` cannot fail and every field has
a working default, so you set only what you care about.

## The fields, grouped

**What the agent is.** `system_prompt` takes a string for a verbatim prompt.
`system_prompt_preset(append, exclude_dynamic_sections)` instead uses the binary's
own `claude_code` prompt and appends to it, which is what you want when the agent
should behave like Claude Code with extra house rules.

**Which model.** `model` overrides the binary's default. `max_tokens` is the
per-request output ceiling and defaults to 4096. `fallback_model` (example 09)
covers the primary being overloaded.

**Tool gating.** Three independent levers:

| Lever | Effect |
|---|---|
| `permission_mode` | how calls are gated: `Default` (ask), `AcceptEdits`, `Plan`, `BypassPermissions`, `DontAsk`, `Auto` |
| `allowed_tools` | the tools offered to the model at all |
| `disallowed_tools` | subtracted from that set |

`Plan` is used here because the example only asks a question — the agent proposes
rather than executing. Example 05 replaces mode-based gating with a policy that
decides each call in Rust.

**Where it runs.** `cwd` is the subprocess working directory, which is also what
the binary treats as the project. `add_dir` grants read access to a path outside it.
`env` adds environment variables for the child only.

**How long.** Two unrelated timeouts:

- `control_request_timeout` bounds one control request's wait for its correlated
  response — `interrupt`, `set_model`, and everything in example 12. Default 60s.
- `shutdown_grace` is how long a graceful exit is allowed before the supervisor
  kills the process. Default 5s.

**Version gating.** The SDK is validated against `claude` 2.0.0 and up. An older
binary warns by default; `require_min_version(true)` makes it a hard
`AgentError::BinaryVersionUnsupported` at connect instead.

## Inspecting what you built

```rust
println!("{options:#?}");
```

`Options` has a hand-written `Debug`. Registered handlers are shown as a count or a
bool — never their contents — so printing options cannot leak what a policy or hook
closed over.
