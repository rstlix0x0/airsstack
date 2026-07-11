---
type: Rust Module
title: clauders::agent::runtime::api::runtime::ApiRuntime
description: ApiRuntime — a Runtime that drives one agent session against the Messages API in-process, running a send/stream/run-tools loop capped at a fixed turn count, gated per tool call by the native permission engine, with prompt-cache breakpoints and a fixed routing identity.
tags: [rust, sdk, agent, runtime, messages-api, native, tool-loop, permissions]
timestamp: 2026-07-11T00:00:00Z
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
    permission_policy: Option<Arc<dyn PermissionPolicy>>,  // Options.permission_policy
    allowed_tools: Vec<String>,                            // Options.allowed_tools, seeds the RuleStore
    interrupt: Arc<AtomicBool>,
    cache_policy: CachePolicy,
    output_format: Option<OutputConfig>,
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
`drive` owns one [`RuleStore`](/crates/clauders/agent/runtime/permission_engine.md)
for the whole call, seeded from `ctx.allowed_tools`
(`RuleStore::new(&ctx.allowed_tools)`) before the loop starts, so rule
updates a policy returns on turn *N* persist into turn *N+1*. It loops up
to `turn_cap` times:

1. Check the interrupt flag; return (silently ending the stream) if set.
2. Build a `MessageRequest` from the running history and the declared tool
   set, applying [`CachePolicy`](/crates/clauders/agent/runtime/api/cache.md)
   breakpoints.
3. `POST` it; a transport/API error is mapped via
   [`convert::map_wire_error`](/crates/clauders/agent/runtime/api/convert.md)
   and sent as the stream's terminal `Err`.
4. Fold the response `Usage` into a running total; emit the assistant turn
   as a `Message::Assistant` frame.
5. If `stop_reason == ToolUse`: run `run_tools(&ctx, &mut store, &response.content)`
   (below); on `ToolLoopStep::Continue`, append the tool-result blocks to
   history and loop; on `ToolLoopStep::Interrupted`, emit the terminal
   `interrupted_result` frame and return.
6. Otherwise emit the terminal `Message::Result` (accumulated usage, mapped
   stop reason, no `total_cost_usd` — the Messages API does not report
   cost) and return.

If the loop exhausts `turn_cap` without a terminal stop reason, it emits an
`is_error: true` `Result` with `stop_reason: "max_turns"` — a finite bound
so a model that never stops calling tools cannot loop forever.

# Permission enforcement (`run_tools`)

```rust
enum ToolLoopStep {
    Continue(Vec<WireBlock>),   // tool-result blocks to feed back to the model
    Interrupted(String),        // turn-aborting deny message
}

async fn run_tools<T: HttpTransport>(
    ctx: &TurnContext<T>,
    store: &mut RuleStore,
    content: &[WireBlock],
) -> ToolLoopStep;

fn apply_input(block: &ToolUseBlock, updated: Option<serde_json::Value>) -> ToolUseBlock;
```

`ToolLoopStep` is an execution-flow signal, not a permission verdict — it
carries no duplicate of `PermissionDecision`'s own information. For every
`ToolUse` block in the turn's response content, `run_tools`:

1. Builds a [`PermissionContext`](/crates/clauders/agent/permissions.md)
   from the block (currently just `tool_use_id`; other context fields are
   `None` — this runtime has no binary-side pre-decision to mirror).
2. Calls [`permission_engine::evaluate`](/crates/clauders/agent/runtime/permission_engine.md)
   with the runtime's current `permission_mode`, the shared `store`, and
   `ctx.permission_policy`. An `Err` from the policy becomes a
   model-visible `ToolResultBlock::err` (not a session failure) and the
   loop continues to the next block — the same non-fatal-error contract
   [`tools::dispatch`](/crates/clauders/agent/runtime/api/tools.md) uses
   for its own failure modes.
3. On `PermissionDecision::Allow { updated_input, .. }`: applies the
   optional input rewrite via `apply_input` (returns the block unchanged
   when `updated_input` is `None`) and dispatches through
   [`tools::dispatch`](/crates/clauders/agent/runtime/api/tools.md).
4. On `PermissionDecision::Deny { interrupt: false, message, .. }`: pushes
   a `ToolResultBlock::err(id, message)` and continues — the tool call
   fails visibly to the model but the turn proceeds.
5. On `PermissionDecision::Deny { interrupt: true, message, .. }`: returns
   `ToolLoopStep::Interrupted(message)` immediately, abandoning any
   remaining tool-use blocks in this response — `drive` maps this to the
   terminal `interrupted_result` frame:
   `stop_reason: "permission_denied"`, `is_error: true`, `result` set to
   the deny message, no `total_cost_usd`, usage accumulated up to the
   interrupt point.

`run_tools` returns `ToolLoopStep::Continue(results)` once every block in
the response has been processed without an interrupting deny (`results`
may be empty only if `content` had no `ToolUse` blocks, which does not
occur when `stop_reason == ToolUse`).

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
[permission_engine (RuleStore/evaluate)](/crates/clauders/agent/runtime/permission_engine.md)
(the enforcement engine `run_tools` consults — see above; documented
there rather than duplicated here),
[permissions module (PermissionMode/PermissionDecision/PermissionPolicy)](/crates/clauders/agent/permissions.md),
[messages/request (MessageRequest)](/crates/clauders/messages/request.md),
[messages/response (Message/Usage/StopReason)](/crates/clauders/messages/response.md),
[RoutingRuntime](/crates/clauders/agent/runtime/routing/runtime.md) (a
typical routing target).

# Citations

1. `crates/clauders/src/agent/runtime/api/runtime.rs`
