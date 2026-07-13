---
type: Rust Module
title: clauders::agent::runtime::openrouter::runtime::OpenRouterRuntime
description: OpenRouterRuntime — a Runtime that drives one agent session against the OpenRouter chat-completions API in-process, with the same send/stream/run-tools/loop shape as ApiRuntime but no prompt caching.
tags: [rust, sdk, agent, runtime, openrouter, native, tool-loop]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/openrouter/runtime.rs
---

Generic over the HTTP transport (defaulting to reqwest), exercisable
offline against a mock transport. Structural twin of
[ApiRuntime](/crates/clauders/agent/runtime/api/runtime.md); part of the
[runtime layer](/crates/clauders/agent/runtime/overview.md).

# Schema

```rust
pub struct OpenRouterRuntime<T: HttpTransport = DefaultTransportPlaceholder> {
    client: OrClient<T>,             // openrouter_rs::Client
    registry: SdkMcpRegistry,
    max_tokens: OrMaxTokens,
    system: Option<String>,          // plain string, unlike ApiRuntime's typed SystemPrompt
    turn_cap: u32,                   // Options.max_turns, default 8
    session_id: SessionId,
    capabilities: Capabilities,
    identity: Option<ModelId>,       // Runtime::model() source
    model: Mutex<OrModelId>,
    permission_mode: Mutex<PermissionMode>,
    interrupt: Arc<AtomicBool>,
}

impl<T: HttpTransport> OpenRouterRuntime<T> {
    pub fn new(client: OrClient<T>, options: Options) -> Result<Self, AgentError>;
}
```

`new` requires `options.model`, converting it to an
`openrouter_rs::types::ModelId` via `OrModelId::custom` — a rejected slug
or an unrepresentable `max_tokens` both fold to `AgentError::Protocol`.
`system` is built the same way as `ApiRuntime`'s: `options.system_prompt`
is a [`SystemPromptConfig`](/crates/clauders/agent/system-prompt.md);
`is_preset()` triggers a `tracing::warn!` ("preset base is unavailable on
OpenRouterRuntime"), then `native_text()` supplies the degraded value —
here a plain `Option<String>`, not a typed wire prompt, since OpenRouter's
chat messages carry system content as an ordinary `system`-role message
string. `capabilities()` is a static manifest (protocol version
`"openrouter-1.0"`; `set_model`, `set_permission_mode`, `interrupt`,
`mcp_status` supported, no hooks) — identical shape to `ApiRuntime`'s.
`model()` returns `self.identity.as_ref()`, fixed at construction.

# Turn loop (`run` / `drive`)

Same shape as [`ApiRuntime`](/crates/clauders/agent/runtime/api/runtime.md):
clear the latched interrupt, spawn `drive` on a `TurnContext`, return an
`mpsc`-backed `MessageStream` immediately. `drive` differs in two
OpenRouter-specific ways:

- History seeds with an `OrMessage::system(..)` turn (when `system` is
  set) ahead of the user turn, rather than a separate request field.
- On `finish_reason == ToolCalls`, it appends an
  `OrMessage::assistant_tool_calls(calls)` turn, then one
  `tools::dispatch` result message per call, before looping — versus
  `ApiRuntime`'s single batched tool-result content block.

Otherwise identical: usage/errors are folded via
[`convert`](/crates/clauders/agent/runtime/openrouter/convert.md), a
non-tool-call finish emits the terminal `Result` (here `total_cost_usd` IS
populated, from `completion.usage.cost` — unlike `ApiRuntime`, whose
Messages-API usage carries no cost field), and `turn_cap` exhaustion emits
an `is_error: true`, `stop_reason: "max_turns"` result.

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::agent::AgentError> {
use clauders::agent::{OpenRouterRuntime, Options};
use clauders::types::ModelId;
use openrouter_rs::Client as OrClient;
use openrouter_rs::types::{ApiKey as OrApiKey, MaxTokens as OrMaxTokens};

let client = OrClient::builder().expect("t").api_key(OrApiKey::new("sk-or-…").expect("k")).build().expect("c");
let opts = Options::builder()
    .model(ModelId::custom("deepseek/deepseek-chat").expect("model"))
    .max_tokens(clauders::types::MaxTokens::new(1024).expect("nz"))
    .build();
let runtime = OpenRouterRuntime::new(client, opts)?;
# let _ = (runtime, OrMaxTokens::new(1));
# Ok(())
# }
```

Related: [runtime module overview](/crates/clauders/agent/runtime/openrouter/overview.md),
[Runtime trait](/crates/clauders/agent/runtime.md),
[SystemPromptConfig](/crates/clauders/agent/system-prompt.md),
[convert](/crates/clauders/agent/runtime/openrouter/convert.md),
[tools](/crates/clauders/agent/runtime/openrouter/tools.md),
[ApiRuntime (structural twin)](/crates/clauders/agent/runtime/api/runtime.md),
[RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md) (a
typical routing target, e.g. the cheap side of a deepseek/claude split).

# Citations

1. `crates/clauders/src/agent/runtime/openrouter/runtime.rs`
