---
type: Rust Module
title: clauders::agent::runtime::api::runtime::ApiRuntime
description: ApiRuntime — a Runtime that drives one agent session against the Messages API in-process, running a send/stream/run-tools loop capped at a fixed turn count, with prompt-cache breakpoints and a fixed routing identity.
tags: [rust, sdk, agent, runtime, messages-api, native, tool-loop]
timestamp: 2026-07-10T00:00:00Z
resource: crates/clauders/src/agent/runtime/api/runtime.rs
---

Generic over the HTTP transport (defaulting to reqwest) so the whole loop
is exercisable offline against a mock transport, mirroring
[`MessagesResource`](/crates/clauders/messages/resource.md). Part of the
[runtime layer](/crates/clauders/agent/runtime/overview.md).

# Schema

```rust
pub struct ApiRuntime<T: HttpTransport = DefaultTransportPlaceholder> {
    client: Client<T>,
    registry: SdkMcpRegistry,
    max_tokens: MaxTokens,
    system: Option<SystemPrompt>,
    turn_cap: u32,               // Options.max_turns, default 8
    session_id: SessionId,
    capabilities: Capabilities,
    identity: Option<ModelId>,   // Runtime::model() source
    model: Mutex<ModelId>,       // mutable current model, set_model()-able
    permission_mode: Mutex<PermissionMode>,
    interrupt: Arc<AtomicBool>,
    cache_policy: CachePolicy,
}

impl<T: HttpTransport> ApiRuntime<T> {
    pub fn new(client: Client<T>, options: Options) -> Result<Self, AgentError>;
    pub const fn with_cache_policy(mut self, policy: CachePolicy) -> Self;
}
```

`new` requires `options.model` to be set — no hidden default — returning
`AgentError::Protocol` otherwise. Its `system` field is built from
[`options.system_prompt.native_text()`](/crates/clauders/agent/system-prompt.md),
logging a `tracing::warn!` first when `options.system_prompt.is_preset()`
(the preset degrades to its `append` alone: no `claude_code` base exists
off the CLI binary). `registry` is `options.sdk_mcp_servers` — the
in-process MCP tool set this runtime declares and dispatches through
[`tools`](/crates/clauders/agent/runtime/api/tools.md). `capabilities()`
reports a static manifest: no hooks, and only `set_model`,
`set_permission_mode`, `interrupt`, `mcp_status` as supported control
methods (protocol version `"api-1.0"`) — `interrupt`/`hook_callback`/etc.
that the CLI's control protocol otherwise carries have no equivalent here.
`model()` returns `self.identity.as_ref()` — the construction-time model,
fixed regardless of later `set_model` calls — this is the identity
[`RoutingRuntimeBuilder`](/crates/clauders/agent/runtime/routing/builder.md)
reads to key this runtime in a routing catalog.

# Turn loop (`run` / `drive`)

`run` clears any latched `interrupt` (a fresh run is never poisoned by a
prior `interrupt()` call), then spawns `drive` on a fresh `TurnContext` and
returns immediately with the receiving end of an
[`mpsc`](/crates/clauders/agent/stream.md) channel as a `MessageStream`.
`drive` loops up to `turn_cap` times:

1. Check the interrupt flag; return (silently ending the stream) if set.
2. Build a `MessageRequest` from the running history and the declared tool
   set, applying [`CachePolicy`](/crates/clauders/agent/runtime/api/cache.md)
   breakpoints.
3. `POST` it; a transport/API error is mapped via
   [`convert::map_wire_error`](/crates/clauders/agent/runtime/api/convert.md)
   and sent as the stream's terminal `Err`.
4. Fold the response `Usage` into a running total; emit the assistant turn
   as a `Message::Assistant` frame.
5. If `stop_reason == ToolUse`: run every tool-use block via
   [`tools::dispatch`](/crates/clauders/agent/runtime/api/tools.md),
   append the results to history, and loop.
6. Otherwise emit the terminal `Message::Result` (accumulated usage, mapped
   stop reason, no `total_cost_usd` — the Messages API does not report
   cost) and return.

If the loop exhausts `turn_cap` without a terminal stop reason, it emits an
`is_error: true` `Result` with `stop_reason: "max_turns"` — a finite bound
so a model that never stops calling tools cannot loop forever.

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::agent::AgentError> {
use clauders::agent::{ApiRuntime, Options};
use clauders::Client as WireClient;
use clauders::types::{ApiKey, MaxTokens, ModelId};

let wire = WireClient::builder().expect("t").api_key(ApiKey::new("sk-ant-…").expect("k")).build().expect("c");
let opts = Options::builder().model(ModelId::claude_sonnet_4_5()).max_tokens(MaxTokens::new(1024).expect("nz")).build();
let runtime = ApiRuntime::new(wire, opts)?;
# let _ = runtime;
# Ok(())
# }
```

Related: [runtime module overview](/crates/clauders/agent/runtime/api/overview.md),
[Runtime trait](/crates/clauders/agent/runtime.md),
[SystemPromptConfig](/crates/clauders/agent/system-prompt.md),
[CachePolicy](/crates/clauders/agent/runtime/api/cache.md),
[convert](/crates/clauders/agent/runtime/api/convert.md),
[tools](/crates/clauders/agent/runtime/api/tools.md),
[messages/request (MessageRequest)](/crates/clauders/messages/request.md),
[messages/response (Message/Usage/StopReason)](/crates/clauders/messages/response.md),
[RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md) (a
typical routing target).

# Citations

1. `crates/clauders/src/agent/runtime/api/runtime.rs`
