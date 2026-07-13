//! The native `Runtime` over the Messages API.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::agent::capabilities::Capabilities;
use crate::agent::content::ContentBlock as AgentBlock;
use crate::agent::error::AgentError;
use crate::agent::mcp::SdkMcpRegistry;
use crate::agent::message::{AssistantMessage, Message, ResultMessage, Usage as AgentUsage};
use crate::agent::options::Options;
use crate::agent::permissions::{
    JudgeRequest, PermissionContext, PermissionDecision, PermissionJudge, PermissionMode,
    PermissionPolicy,
};
use crate::agent::runtime::Runtime;
use crate::agent::runtime::permission_engine::{self, RuleStore};
use crate::agent::stream::{MessageStream, ReceiverStream};
use crate::agent::subagents::AgentDefinition;
use crate::agent::types::{McpStatus, Prompt, ServerStatus, SessionControl, SessionId};
use crate::client::Client;
use crate::client::DefaultTransportPlaceholder;
use crate::messages::content::ContentBlock as WireBlock;
use crate::messages::request::{InputMessage, MessageContent, MessageRequest, Role};
use crate::messages::response::{Message as WireMessage, StopReason, Usage as WireUsage};
use crate::messages::structured_outputs::OutputConfig;
use crate::messages::tools::{Tool as WireTool, ToolResultBlock, ToolUseBlock};
use crate::transport::HttpTransport;
use crate::types::{MaxTokens, ModelId, SystemPrompt};

use super::cache::CachePolicy;
use super::session_store::{self, SessionIds, SessionSink, SessionStore};
use super::subagent::{self, ToolFilter};
use super::{cache, convert, tools};

/// Per-turn message channel capacity (natural backpressure beyond this).
const TURN_CHANNEL_CAPACITY: usize = 64;
/// Turn cap applied when `Options.max_turns` is unset — a finite bound so a
/// model that never stops calling tools cannot loop forever.
const DEFAULT_MAX_TURNS: u32 = 8;
/// The static control-protocol version this runtime reports.
const PROTOCOL_VERSION: &str = "api-1.0";

/// Monotonic source of per-runtime session identifiers.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Mint a fresh, process-unique session id for a new or forked session.
fn mint_session_id() -> SessionId {
    let n = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    SessionId::new(format!("api-session-{nanos}-{n}"))
}

/// Resolve a [`SessionControl`] against the store into the load source and
/// write target. Errors only when a `Resume` names an id absent from the
/// store (fail closed, matching the CLI's "no conversation found").
fn resolve_session(
    control: &SessionControl,
    store: &SessionStore,
) -> Result<SessionIds, AgentError> {
    Ok(match control {
        SessionControl::New => {
            let id = mint_session_id();
            SessionIds::new(id.clone(), id)
        }
        SessionControl::Continue { fork } => {
            let source = store.most_recent().unwrap_or_else(mint_session_id);
            if *fork {
                SessionIds::new(source, mint_session_id())
            } else {
                SessionIds::new(source.clone(), source)
            }
        }
        SessionControl::Resume { id, fork } => {
            if !store.exists(id) {
                return Err(AgentError::SessionStore {
                    detail: format!("no conversation found with session id `{id}`"),
                });
            }
            if *fork {
                SessionIds::new(id.clone(), mint_session_id())
            } else {
                SessionIds::new(id.clone(), id.clone())
            }
        }
    })
}

/// A [`Runtime`] that drives one agent session against the Messages API.
///
/// Generic over the HTTP transport (defaulting to reqwest) so the whole loop
/// is exercisable offline against a mock transport, mirroring
/// [`crate::messages::MessagesResource`].
pub struct ApiRuntime<T: HttpTransport = DefaultTransportPlaceholder> {
    client: Client<T>,
    registry: SdkMcpRegistry,
    max_tokens: MaxTokens,
    system: Option<SystemPrompt>,
    turn_cap: u32,
    capabilities: Capabilities,
    identity: Option<ModelId>,
    model: Mutex<ModelId>,
    permission_mode: Mutex<PermissionMode>,
    permission_policy: Option<Arc<dyn PermissionPolicy>>,
    permission_judge: Option<Arc<dyn PermissionJudge>>,
    allowed_tools: Vec<String>,
    interrupt: std::sync::Arc<AtomicBool>,
    cache_policy: CachePolicy,
    output_format: Option<OutputConfig>,
    agents: HashMap<String, AgentDefinition>,
    store: SessionStore,
    session: Arc<Mutex<SessionIds>>,
}

impl<T: HttpTransport> ApiRuntime<T> {
    /// Build a runtime from a wire [`Client`] and session [`Options`].
    ///
    /// # Errors
    /// Returns [`AgentError::Protocol`] when `Options.model` is `None`: the
    /// Messages API requires an explicit model and this runtime applies no
    /// hidden default.
    pub fn new(client: Client<T>, options: Options) -> Result<Self, AgentError> {
        let model = options.model.ok_or_else(|| AgentError::Protocol {
            detail: "ApiRuntime requires Options.model to be set".to_string(),
        })?;
        let store = SessionStore::new(
            options
                .session_dir
                .clone()
                .unwrap_or_else(session_store::default_root),
        );
        Ok(Self {
            client,
            registry: options.sdk_mcp_servers,
            max_tokens: options.max_tokens,
            system: {
                if options.system_prompt.is_preset() {
                    tracing::warn!(
                        "system-prompt preset base is unavailable on ApiRuntime; using append only"
                    );
                }
                options.system_prompt.native_text().map(SystemPrompt::text)
            },
            turn_cap: options.max_turns.unwrap_or(DEFAULT_MAX_TURNS),
            capabilities: build_capabilities(),
            identity: Some(model.clone()),
            model: Mutex::new(model),
            permission_mode: Mutex::new(options.permission_mode),
            permission_policy: options.permission_policy,
            permission_judge: options.permission_judge,
            allowed_tools: options.allowed_tools,
            interrupt: std::sync::Arc::new(AtomicBool::new(false)),
            cache_policy: CachePolicy::default(),
            output_format: options.output_format,
            agents: options.agents,
            session: Arc::new(Mutex::new(resolve_session(&options.session, &store)?)),
            store,
        })
    }

    /// Set the prompt-cache policy for this runtime, consuming and returning it.
    ///
    /// Defaults to [`CachePolicy::PrefixAndConversation`]; call this to override.
    #[must_use]
    pub const fn with_cache_policy(mut self, policy: CachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    /// Enumerate the ids of sessions persisted in this runtime's store.
    ///
    /// # Errors
    /// Returns [`AgentError::SessionStore`] if the store directory exists
    /// but cannot be read.
    pub fn list_sessions(&self) -> Result<Vec<SessionId>, AgentError> {
        self.store.list()
    }

    /// Read the current model, recovering from a poisoned lock.
    fn current_model(&self) -> ModelId {
        self.model
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Read the current permission mode, recovering from a poisoned lock.
    fn current_permission_mode(&self) -> PermissionMode {
        *self
            .permission_mode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The static capability manifest: no hooks, only the control methods this
/// runtime genuinely honors.
fn build_capabilities() -> Capabilities {
    let mut caps = Capabilities {
        protocol_version: PROTOCOL_VERSION.to_string(),
        ..Capabilities::default()
    };
    for method in [
        "set_model",
        "set_permission_mode",
        "interrupt",
        "mcp_status",
    ] {
        caps.supported_control_methods.insert(method.to_string());
    }
    caps
}

#[async_trait]
impl<T: HttpTransport> Runtime for ApiRuntime<T> {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
        // Each run starts fresh: clear any interrupt latched by a prior turn so
        // a reused runtime is never permanently poisoned by one `interrupt()`.
        self.interrupt.store(false, Ordering::SeqCst);

        // Snapshot the load source and write target. The load pointer is NOT
        // collapsed here — that happens only after a successful persist (in
        // `SessionSink::save`), so a fork whose first run fails before the
        // terminal persist stays re-seedable from its source on retry.
        let (load_id, write_id) = {
            let ids = self
                .session
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (ids.load.clone(), ids.write.clone())
        };
        let history_seed = self.store.load(&load_id)?;

        let (tx, rx) = mpsc::channel::<Result<Message, AgentError>>(TURN_CHANNEL_CAPACITY);
        let ctx = TurnContext {
            client: self.client.clone(),
            registry: self.registry.clone(),
            model: self.current_model(),
            max_tokens: self.max_tokens,
            system: self.system.clone(),
            turn_cap: self.turn_cap,
            session_id: write_id.clone(),
            interrupt: std::sync::Arc::clone(&self.interrupt),
            cache_policy: self.cache_policy,
            permission_mode: self.current_permission_mode(),
            permission_policy: self.permission_policy.clone(),
            permission_judge: self.permission_judge.clone(),
            allowed_tools: self.allowed_tools.clone(),
            output_format: self.output_format.clone(),
            agents: self.agents.clone(),
            tool_filter: ToolFilter::inherit_all(),
            prior_history: history_seed,
            session_sink: Some(SessionSink::new(
                self.store.clone(),
                write_id,
                std::sync::Arc::clone(&self.session),
            )),
        };
        tokio::spawn(drive(ctx, prompt, tx));
        Ok(ReceiverStream::new(rx).boxed())
    }

    async fn interrupt(&self) -> Result<(), AgentError> {
        self.interrupt.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
        *self
            .model
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = model;
        Ok(())
    }

    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
        *self
            .permission_mode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = mode;
        Ok(())
    }

    async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
        let servers = self
            .registry
            .servers()
            .map(|s| ServerStatus {
                name: s.name().to_string(),
                status: "connected".to_string(),
            })
            .collect();
        Ok(McpStatus { servers })
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn model(&self) -> Option<&ModelId> {
        self.identity.as_ref()
    }
}

/// Everything one `run` turn-loop owns, moved into the spawned task.
struct TurnContext<T: HttpTransport> {
    client: Client<T>,
    registry: SdkMcpRegistry,
    model: ModelId,
    max_tokens: MaxTokens,
    system: Option<SystemPrompt>,
    turn_cap: u32,
    session_id: SessionId,
    interrupt: std::sync::Arc<AtomicBool>,
    cache_policy: CachePolicy,
    permission_mode: PermissionMode,
    permission_policy: Option<Arc<dyn PermissionPolicy>>,
    permission_judge: Option<Arc<dyn PermissionJudge>>,
    allowed_tools: Vec<String>,
    output_format: Option<OutputConfig>,
    agents: HashMap<String, AgentDefinition>,
    tool_filter: ToolFilter,
    prior_history: Vec<InputMessage>,
    session_sink: Option<SessionSink>,
}

/// Drive the agent loop, pushing frames into `tx` until the turn ends, the
/// turn cap is hit, an error occurs, or an interrupt is observed.
///
/// Returns a boxed, pinned future rather than an `async fn`'s opaque type:
/// `run_tools` may route an `Agent` call back into this same function
/// (`run_subagent`), and a self-referential opaque type cannot be named.
/// Boxing here gives the recursion a concrete, `Send`-declared type to close
/// the cycle on.
fn drive<T: HttpTransport>(
    ctx: TurnContext<T>,
    prompt: Prompt,
    tx: mpsc::Sender<Result<Message, AgentError>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(drive_inner(ctx, prompt, tx))
}

/// The actual turn-loop body, boxed by [`drive`].
async fn drive_inner<T: HttpTransport>(
    ctx: TurnContext<T>,
    prompt: Prompt,
    tx: mpsc::Sender<Result<Message, AgentError>>,
) {
    let mut tool_defs: Vec<WireTool> = ctx.tool_filter.apply(tools::declare(&ctx.registry));
    if !ctx.agents.is_empty() {
        if let Some(agent_tool) = subagent::declare_agent_tool(&ctx.agents) {
            tool_defs.push(agent_tool);
        }
    }
    let mut history: Vec<InputMessage> = ctx.prior_history.clone();
    history.push(InputMessage {
        role: Role::User,
        content: MessageContent::Text(prompt.as_str().to_string()),
    });
    let mut usage_total = AgentUsage::default();
    let mut store = RuleStore::new(&ctx.allowed_tools);

    for turn in 0..ctx.turn_cap {
        if ctx.interrupt.load(Ordering::SeqCst) {
            return;
        }

        let request = build_request(&ctx, &history, &tool_defs);
        let response = match ctx.client.messages().create(request).await {
            Ok(response) => response,
            Err(error) => {
                let _ = tx.send(Err(convert::map_wire_error(error))).await;
                return;
            }
        };
        accumulate_usage(&mut usage_total, &response.usage);

        if emit_assistant(&tx, &response).await.is_err() {
            return; // receiver dropped
        }
        history.push(InputMessage {
            role: Role::Assistant,
            content: MessageContent::Blocks(response.content.clone()),
        });

        if response.stop_reason == Some(StopReason::ToolUse) {
            match run_tools(&ctx, &mut store, Some(prompt.as_str()), &response.content).await {
                ToolLoopStep::Continue(results) => {
                    history.push(InputMessage {
                        role: Role::User,
                        content: MessageContent::Blocks(results),
                    });
                    continue;
                }
                ToolLoopStep::Interrupted(message) => {
                    let _ = tx
                        .send(Ok(interrupted_result(
                            &ctx.session_id,
                            turn + 1,
                            message,
                            usage_total,
                        )))
                        .await;
                    return;
                }
            }
        }

        let _ = tx
            .send(Ok(terminal_result(
                &ctx,
                &response,
                turn + 1,
                false,
                usage_total,
            )))
            .await;
        persist_session(&ctx, &history, &tx).await;
        return;
    }

    // Turn cap exhausted without a terminal stop reason.
    let _ = tx
        .send(Ok(exhausted_result(
            &ctx.session_id,
            ctx.turn_cap,
            usage_total,
        )))
        .await;
}

/// Persist the completed conversation to the session store, if this run has
/// a sink. A save failure is surfaced as a trailing error frame — the turns
/// already streamed; only the durable record failed.
async fn persist_session<T: HttpTransport>(
    ctx: &TurnContext<T>,
    history: &[InputMessage],
    tx: &mpsc::Sender<Result<Message, AgentError>>,
) {
    if let Some(sink) = &ctx.session_sink {
        if let Err(error) = sink.save(history) {
            let _ = tx.send(Err(error)).await;
        }
    }
}

/// Fold one wire response's usage into the running per-run total.
fn accumulate_usage(total: &mut AgentUsage, wire: &WireUsage) {
    let turn = convert::usage(wire);
    total.input_tokens += turn.input_tokens;
    total.output_tokens += turn.output_tokens;
    if let Some(created) = turn.cache_creation_input_tokens {
        total.cache_creation_input_tokens =
            Some(total.cache_creation_input_tokens.unwrap_or(0) + created);
    }
    if let Some(read) = turn.cache_read_input_tokens {
        total.cache_read_input_tokens = Some(total.cache_read_input_tokens.unwrap_or(0) + read);
    }
}

/// Build the next `MessageRequest` from the running history and tool set,
/// applying the runtime's prompt-cache breakpoints.
fn build_request<T: HttpTransport>(
    ctx: &TurnContext<T>,
    history: &[InputMessage],
    tool_defs: &[WireTool],
) -> MessageRequest {
    let mut system = ctx.system.clone();
    let mut tools: Vec<WireTool> = tool_defs.to_vec();
    cache::apply_prefix(ctx.cache_policy, &mut system, &mut tools);

    let mut history: Vec<InputMessage> = history.to_vec();
    cache::apply_conversation(ctx.cache_policy, &mut history);

    let mut builder = MessageRequest::builder()
        .model(ctx.model.clone())
        .max_tokens(ctx.max_tokens);
    if let Some(system) = system {
        builder = builder.system(system);
    }
    for message in history {
        builder = builder.add_message(message.role, message.content);
    }
    let mut builder = builder.tools(tools);
    if let Some(config) = &ctx.output_format {
        builder = builder.output_config(config.clone());
    }
    builder.build()
}

/// Emit the assistant turn as an agent frame.
async fn emit_assistant(
    tx: &mpsc::Sender<Result<Message, AgentError>>,
    response: &WireMessage,
) -> Result<(), ()> {
    let content: Vec<AgentBlock> = response
        .content
        .iter()
        .map(convert::content_block)
        .collect();
    tx.send(Ok(Message::Assistant(AssistantMessage {
        content,
        parent_tool_use_id: None,
    })))
    .await
    .map_err(|_| ())
}

/// The outcome of running one turn's tool-use blocks: either the tool-result
/// blocks to feed back to the model, or a turn-aborting interrupt message. This
/// is an execution-flow signal, not a permission verdict — no duplicate type.
enum ToolLoopStep {
    Continue(Vec<WireBlock>),
    Interrupted(String),
}

/// The assistant message's own text, joined, as the agent's stated rationale
/// for the tool calls in that message. `None` when there is no text block, or
/// when the joined text is empty or whitespace-only.
fn message_rationale(content: &[WireBlock]) -> Option<String> {
    let text = content
        .iter()
        .filter_map(|block| match block {
            WireBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    (!text.trim().is_empty()).then_some(text)
}

/// Gate and run every tool-use block in `content`. Each call is evaluated
/// against `store` (session rules), the runtime's mode, and its policy:
/// an allow dispatches the tool (with any rewritten input), a non-interrupt
/// deny yields a model-visible error result and the loop continues, and an
/// interrupt deny aborts the turn.
async fn run_tools<T: HttpTransport>(
    ctx: &TurnContext<T>,
    store: &mut RuleStore,
    task: Option<&str>,
    content: &[WireBlock],
) -> ToolLoopStep {
    let mut results = Vec::new();
    let rationale = message_rationale(content);
    for block in content {
        let WireBlock::ToolUse(use_block) = block else {
            continue;
        };
        let pctx = PermissionContext {
            tool_use_id: Some(use_block.id.as_str().to_string()),
            ..PermissionContext::default()
        };
        let review = JudgeRequest {
            tool: use_block.name.as_str(),
            input: &use_block.input,
            task,
            rationale: rationale.as_deref(),
            ctx: &pctx,
        };
        let decision = match permission_engine::evaluate(
            ctx.permission_mode,
            store,
            ctx.permission_judge.as_ref(),
            ctx.permission_policy.as_ref(),
            &review,
        )
        .await
        {
            Ok(decision) => decision,
            Err(error) => {
                // A policy failure becomes a model-visible error result rather
                // than a session failure; the loop continues.
                results.push(WireBlock::ToolResult(ToolResultBlock::err(
                    use_block.id.clone(),
                    error.to_string(),
                )));
                continue;
            }
        };
        match decision {
            PermissionDecision::Allow { updated_input, .. } => {
                let gated = apply_input(use_block, updated_input);
                let result = if subagent::is_agent_tool(gated.name.as_str()) {
                    run_subagent(ctx, &gated).await
                } else {
                    tools::dispatch(&ctx.registry, &gated).await
                };
                results.push(WireBlock::ToolResult(result));
            }
            PermissionDecision::Deny {
                message,
                interrupt: false,
                ..
            } => {
                results.push(WireBlock::ToolResult(ToolResultBlock::err(
                    use_block.id.clone(),
                    message,
                )));
            }
            PermissionDecision::Deny {
                message,
                interrupt: true,
                ..
            } => {
                return ToolLoopStep::Interrupted(message);
            }
        }
    }
    ToolLoopStep::Continue(results)
}

/// Apply a decision's optional input rewrite to a tool-use block.
fn apply_input(block: &ToolUseBlock, updated: Option<serde_json::Value>) -> ToolUseBlock {
    updated.map_or_else(
        || block.clone(),
        |input| ToolUseBlock {
            input,
            ..block.clone()
        },
    )
}

/// Derive a child turn context for a subagent: its own model/prompt/tool
/// filter/turn cap/permission mode (each inheriting the parent when unset),
/// the parent's inherited judge and policy, and `agents = ∅` so the child
/// is offered no `Agent` tool (depth-1).
fn child_context<T: HttpTransport>(
    parent: &TurnContext<T>,
    def: &AgentDefinition,
) -> TurnContext<T> {
    let n = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    TurnContext {
        client: parent.client.clone(),
        registry: parent.registry.clone(),
        model: def.model().cloned().unwrap_or_else(|| parent.model.clone()),
        max_tokens: parent.max_tokens,
        system: Some(SystemPrompt::text(def.prompt())),
        turn_cap: def.max_turns().unwrap_or(parent.turn_cap),
        session_id: SessionId::new(format!("api-subagent-{n}")),
        interrupt: std::sync::Arc::clone(&parent.interrupt),
        cache_policy: parent.cache_policy,
        permission_mode: def.permission_mode().unwrap_or(parent.permission_mode),
        permission_policy: parent.permission_policy.clone(),
        permission_judge: parent.permission_judge.clone(),
        allowed_tools: parent.allowed_tools.clone(),
        output_format: None,
        agents: HashMap::new(),
        tool_filter: ToolFilter::from_definition(def),
        prior_history: Vec::new(),
        session_sink: None,
    }
}

/// Run a registered subagent as an isolated nested loop and return its final
/// message as the `Agent` tool result. Intermediate child frames are drained
/// internally (never forwarded to the parent stream) — context isolation.
async fn run_subagent<T: HttpTransport>(
    ctx: &TurnContext<T>,
    block: &ToolUseBlock,
) -> ToolResultBlock {
    let id = block.id.clone();
    let Some((name, prompt)) = subagent::parse_agent_call(&block.input) else {
        return ToolResultBlock::err(id, "Agent call missing subagent_type or prompt".to_string());
    };
    let Some(def) = ctx.agents.get(&name) else {
        return ToolResultBlock::err(id, format!("unknown subagent: {name}"));
    };
    let child = child_context(ctx, def);
    let (tx, mut rx) = mpsc::channel::<Result<Message, AgentError>>(TURN_CHANNEL_CAPACITY);
    tokio::spawn(drive(child, Prompt::new(prompt), tx));
    let mut last_result: Option<ResultMessage> = None;
    while let Some(item) = rx.recv().await {
        match item {
            Ok(Message::Result(result)) => last_result = Some(result),
            Ok(_) => {} // intermediate assistant/tool frames stay inside the child
            Err(error) => return ToolResultBlock::err(id, error.to_string()),
        }
    }
    match last_result {
        Some(result) if result.is_error => ToolResultBlock::err(id, result.result),
        Some(result) if result.result.trim().is_empty() => {
            ToolResultBlock::err(id, "subagent finished without output".to_string())
        }
        Some(result) => ToolResultBlock::text(id, result.result),
        None => ToolResultBlock::err(id, "subagent produced no result".to_string()),
    }
}

/// The terminal `Result` frame for a completed turn.
fn terminal_result<T: HttpTransport>(
    ctx: &TurnContext<T>,
    response: &WireMessage,
    num_turns: u32,
    is_error: bool,
    usage: AgentUsage,
) -> Message {
    let result = convert::last_text(&response.content);
    let structured_output = ctx
        .output_format
        .as_ref()
        .and_then(|_| serde_json::from_str::<serde_json::Value>(&result).ok());
    Message::Result(ResultMessage {
        result,
        structured_output,
        is_error,
        total_cost_usd: None,
        stop_reason: response
            .stop_reason
            .map(|r| convert::stop_reason_wire(r).to_string()),
        usage: Some(usage),
        session_id: ctx.session_id.clone(),
        num_turns,
    })
}

/// The terminal `Result` frame when the turn cap is exhausted.
fn exhausted_result(session_id: &SessionId, turn_cap: u32, usage: AgentUsage) -> Message {
    Message::Result(ResultMessage {
        result: String::new(),
        structured_output: None,
        is_error: true,
        total_cost_usd: None,
        stop_reason: Some("max_turns".to_string()),
        usage: Some(usage),
        session_id: session_id.clone(),
        num_turns: turn_cap,
    })
}

/// The terminal `Result` frame when a permission policy interrupts the turn.
fn interrupted_result(
    session_id: &SessionId,
    num_turns: u32,
    message: String,
    usage: AgentUsage,
) -> Message {
    Message::Result(ResultMessage {
        result: message,
        structured_output: None,
        is_error: true,
        total_cost_usd: None,
        stop_reason: Some("permission_denied".to_string()),
        usage: Some(usage),
        session_id: session_id.clone(),
        num_turns,
    })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use std::pin::Pin;
    use std::task::{Context, Poll};

    use bytes::Bytes;
    use futures_core::Stream;
    use futures_util::StreamExt;
    use http::{Response, StatusCode};

    use super::ApiRuntime;
    use crate::agent::error::AgentError;
    use crate::agent::message::Message;
    use crate::agent::options::Options;
    use crate::agent::runtime::Runtime;
    use crate::client::Client;
    use crate::error::TransportError;
    use crate::test_support::MockHttpTransport;
    use crate::transport::BodyStream;
    use crate::types::{ApiKey, MaxTokens, ModelId};

    fn body(payload: &str) -> BodyStream {
        struct Once(Option<Bytes>);
        impl Stream for Once {
            type Item = Result<Bytes, TransportError>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                Poll::Ready(self.0.take().map(Ok))
            }
        }
        Box::pin(Once(Some(Bytes::from(payload.to_owned()))))
    }

    fn ok_response(
        payload: &'static str,
    ) -> impl FnMut(http::Request<Bytes>) -> Result<Response<BodyStream>, TransportError> {
        move |_req| {
            let mut resp = Response::new(body(payload));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        }
    }

    fn options_with_model() -> Options {
        Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .build()
    }

    fn client_with(transport: MockHttpTransport) -> Client<MockHttpTransport> {
        Client::builder_with_transport(transport)
            .api_key(ApiKey::new("sk-test").unwrap())
            .build()
            .unwrap()
    }

    const END_TURN: &str = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"hello"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":1}}"#;

    #[test]
    fn new_requires_a_model() {
        let client = client_with(MockHttpTransport::new());
        let opts = Options::builder().build(); // model is None
        assert!(ApiRuntime::new(client, opts).is_err());
    }

    #[test]
    fn capabilities_report_honored_control_methods() {
        let rt = ApiRuntime::new(client_with(MockHttpTransport::new()), options_with_model())
            .expect("runtime");
        let caps = rt.capabilities();
        assert!(caps.supports_control("set_model"));
        assert!(caps.supported_hook_events.is_empty());
    }

    #[tokio::test]
    async fn mcp_status_reports_registered_servers() {
        use crate::agent::mcp::SdkMcpServer;
        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .sdk_mcp_server(SdkMcpServer::builder("calc").build())
            .build();
        let rt = ApiRuntime::new(client_with(MockHttpTransport::new()), opts).expect("runtime");
        let status = rt.mcp_status().await.expect("status");
        assert_eq!(status.servers.len(), 1);
        assert_eq!(status.servers[0].status, "connected");
    }

    #[tokio::test]
    async fn single_turn_streams_assistant_then_result() {
        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(1)
            .returning(ok_response(END_TURN));
        let rt = ApiRuntime::new(client_with(transport), options_with_model()).expect("runtime");
        let mut stream = rt.run("hi".into()).await.expect("run");

        let first = stream.next().await.expect("assistant").expect("ok");
        assert!(matches!(first, Message::Assistant(a) if a.content.len() == 1));
        let second = stream.next().await.expect("result").expect("ok");
        match second {
            Message::Result(r) => {
                assert_eq!(r.result, "hello");
                assert!(!r.is_error);
                assert_eq!(r.num_turns, 1);
                assert_eq!(r.stop_reason.as_deref(), Some("end_turn"));
            }
            other => panic!("expected Result, got {other:?}"),
        }
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn set_model_updates_the_next_request_model() {
        let rt = ApiRuntime::new(client_with(MockHttpTransport::new()), options_with_model())
            .expect("runtime");
        rt.set_model(ModelId::custom("claude-opus-4-8").unwrap())
            .await
            .expect("set_model");
        // No panic and Ok is the observable contract for the local-state mutation.
    }

    #[tokio::test]
    async fn run_after_interrupt_is_not_poisoned() {
        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(1)
            .returning(ok_response(END_TURN));
        let rt = ApiRuntime::new(client_with(transport), options_with_model()).expect("runtime");
        rt.interrupt().await.expect("interrupt");
        // A fresh run clears the latched interrupt and completes normally.
        let mut stream = rt.run("hi".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => {
                assert!(!r.is_error);
                assert_eq!(r.stop_reason.as_deref(), Some("end_turn"));
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    use crate::agent::mcp::SdkMcpServer;
    use crate::agent::mcp::tool::{ToolResult, tool};

    fn calc_options() -> Options {
        let add = tool(
            "add",
            "Add two ints",
            serde_json::json!({"type": "object"}),
            |args| async move {
                let s = args["a"].as_i64().unwrap_or(0) + args["b"].as_i64().unwrap_or(0);
                Ok(ToolResult::text(s.to_string()))
            },
        );
        Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .sdk_mcp_server(SdkMcpServer::builder("calc").tool(add).build())
            .build()
    }

    const TOOL_TURN: &str = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"tool_use","id":"toolu_1","name":"mcp__calc__add","input":{"a":2,"b":3}}],"stop_reason":"tool_use","stop_sequence":null,"usage":{"input_tokens":5,"output_tokens":2}}"#;
    const FINAL_TURN: &str = r#"{"id":"msg_2","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"the answer is 5"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":9,"output_tokens":4}}"#;

    #[tokio::test]
    async fn tool_use_round_trip_then_end_turn() {
        let mut transport = MockHttpTransport::new();
        let mut turn = 0u8;
        transport.expect_send().times(2).returning(move |_req| {
            turn += 1;
            let payload = if turn == 1 { TOOL_TURN } else { FINAL_TURN };
            let mut resp = Response::new(body(payload));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });
        let rt = ApiRuntime::new(client_with(transport), calc_options()).expect("runtime");
        let mut stream = rt.run("add 2 and 3".into()).await.expect("run");

        let frames: Vec<Message> = collect(&mut stream).await;
        // assistant(tool_use), assistant(text), result
        assert_eq!(frames.len(), 3);
        match frames.last().expect("result") {
            Message::Result(r) => {
                assert_eq!(r.result, "the answer is 5");
                assert_eq!(r.num_turns, 2);
                assert!(!r.is_error);
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    fn record_request_models(
        slot: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
        payloads: Vec<&'static str>,
    ) -> impl FnMut(http::Request<Bytes>) -> Result<Response<BodyStream>, TransportError> {
        let mut call = 0usize;
        move |req| {
            let parsed: serde_json::Value =
                serde_json::from_slice(req.body()).unwrap_or(serde_json::Value::Null);
            slot.lock().unwrap().push(parsed);
            let payload = payloads[call.min(payloads.len() - 1)];
            call += 1;
            let mut resp = Response::new(body(payload));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        }
    }

    fn options_with_subagent(name: &str, def: crate::agent::subagents::AgentDefinition) -> Options {
        Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .allowed_tools(vec!["Agent".to_string()])
            .agent(name, def)
            .build()
    }

    #[tokio::test]
    async fn agent_tool_is_declared_when_subagents_registered() {
        use crate::agent::subagents::AgentDefinition;
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(1)
            .returning(record_request_models(requests.clone(), vec![END_TURN]));
        let def = AgentDefinition::new("cheap helper", "be brief").expect("valid");
        let rt = ApiRuntime::new(client_with(transport), options_with_subagent("cheap", def))
            .expect("runtime");
        let mut stream = rt.run("hi".into()).await.expect("run");
        let _ = collect(&mut stream).await;

        let reqs = requests.lock().unwrap().clone();
        let tools = reqs[0]["tools"].as_array().expect("tools array");
        assert!(
            tools.iter().any(|t| t["name"] == "Agent"),
            "Agent tool must be declared when agents are registered"
        );
    }

    const AGENT_CALL_TURN: &str = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"tool_use","id":"toolu_1","name":"Agent","input":{"subagent_type":"cheap","prompt":"do the subtask"}}],"stop_reason":"tool_use","stop_sequence":null,"usage":{"input_tokens":5,"output_tokens":2}}"#;
    const CHILD_END_TURN: &str = r#"{"id":"msg_c","type":"message","role":"assistant","model":"claude-haiku-4-5","content":[{"type":"text","text":"child done"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":4,"output_tokens":2}}"#;
    const PARENT_FINAL_TURN: &str = r#"{"id":"msg_2","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"parent done"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":9,"output_tokens":4}}"#;

    #[tokio::test]
    async fn agent_call_runs_child_on_its_own_model_then_parent_resumes() {
        use crate::agent::subagents::AgentDefinition;
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut transport = MockHttpTransport::new();
        // 3 requests: parent(tool_use Agent) → child(end_turn) → parent(end_turn).
        transport
            .expect_send()
            .times(3)
            .returning(record_request_models(
                requests.clone(),
                vec![AGENT_CALL_TURN, CHILD_END_TURN, PARENT_FINAL_TURN],
            ));
        let def = AgentDefinition::new("cheap helper", "be brief")
            .expect("valid")
            .with_model(ModelId::claude_haiku_4_5());
        let rt = ApiRuntime::new(client_with(transport), options_with_subagent("cheap", def))
            .expect("runtime");
        let mut stream = rt.run("do it".into()).await.expect("run");
        let frames = collect(&mut stream).await;

        // Parent finished normally with its own final text.
        match frames.last().expect("terminal") {
            Message::Result(r) => assert_eq!(r.result, "parent done"),
            other => panic!("expected Result, got {other:?}"),
        }

        let reqs = requests.lock().unwrap().clone();
        assert_eq!(reqs.len(), 3, "parent, child, parent");
        // Downgrade: parent on advanced model, child on the cheap model.
        assert_eq!(reqs[0]["model"], "claude-sonnet-4-5");
        assert_eq!(reqs[1]["model"], "claude-haiku-4-5");
        assert_eq!(reqs[2]["model"], "claude-sonnet-4-5");
        // Depth-1: the child request is offered no Agent tool. The wire
        // request omits an empty `tools` array entirely (skip_serializing_if
        // = Vec::is_empty), so absence of the key is itself proof of no tools.
        let child_has_agent_tool = reqs[1]
            .get("tools")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|tools| tools.iter().any(|t| t["name"] == "Agent"));
        assert!(
            !child_has_agent_tool,
            "a subagent must not be offered the Agent tool (depth-1)"
        );
        // The child runs on the definition's prompt as its system prompt.
        assert!(
            reqs[1].to_string().contains("be brief"),
            "child request must carry def.prompt as its system prompt"
        );
        // The child's final message text is returned verbatim to the parent as
        // the Agent tool result — it appears in the parent's resume request.
        assert!(
            reqs[2].to_string().contains("child done"),
            "child final text must be fed back as the parent's Agent tool result"
        );
    }

    #[tokio::test]
    async fn agent_call_with_unknown_subagent_is_an_error_result_and_loop_continues() {
        use crate::agent::subagents::AgentDefinition;
        // Parent calls Agent for a name that is NOT registered → error result,
        // no child request, parent still ends normally. Only 2 requests.
        let unknown_call = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"tool_use","id":"toolu_1","name":"Agent","input":{"subagent_type":"nope","prompt":"x"}}],"stop_reason":"tool_use","stop_sequence":null,"usage":{"input_tokens":5,"output_tokens":2}}"#;
        let mut transport = MockHttpTransport::new();
        let mut turn = 0u8;
        transport.expect_send().times(2).returning(move |_req| {
            turn += 1;
            let payload = if turn == 1 {
                unknown_call
            } else {
                PARENT_FINAL_TURN
            };
            let mut resp = Response::new(body(payload));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });
        let def = AgentDefinition::new("cheap helper", "be brief").expect("valid");
        let rt = ApiRuntime::new(client_with(transport), options_with_subagent("cheap", def))
            .expect("runtime");
        let mut stream = rt.run("do it".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => {
                assert!(!r.is_error);
                assert_eq!(r.result, "parent done");
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    use std::sync::atomic::{AtomicBool, Ordering};

    use async_trait::async_trait;

    use crate::agent::error::AgentError as AgentErr;
    use crate::agent::permissions::{
        JudgeRequest, PermissionContext, PermissionDecision, PermissionJudge, PermissionMode,
        PermissionPolicy,
    };

    fn ran_flag_options(
        ran: std::sync::Arc<AtomicBool>,
        mode: PermissionMode,
        allow: &[&str],
    ) -> Options {
        let add = tool(
            "add",
            "Add two ints",
            serde_json::json!({"type": "object"}),
            move |_args| {
                let ran_for_tool = ran.clone();
                async move {
                    ran_for_tool.store(true, Ordering::SeqCst);
                    Ok(ToolResult::text("5"))
                }
            },
        );
        Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .permission_mode(mode)
            .allowed_tools(allow.iter().map(|s| (*s).to_string()).collect())
            .sdk_mcp_server(SdkMcpServer::builder("calc").tool(add).build())
            .build()
    }

    fn two_turn_transport() -> MockHttpTransport {
        let mut transport = MockHttpTransport::new();
        let mut turn = 0u8;
        transport.expect_send().times(2).returning(move |_req| {
            turn += 1;
            let payload = if turn == 1 { TOOL_TURN } else { FINAL_TURN };
            let mut resp = Response::new(body(payload));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });
        transport
    }

    #[tokio::test]
    async fn dont_ask_denies_unapproved_tool_but_loop_continues() {
        let ran = std::sync::Arc::new(AtomicBool::new(false));
        // Tool "mcp__calc__add" is NOT in allowed_tools → DontAsk denies it.
        let opts = ran_flag_options(ran.clone(), PermissionMode::DontAsk, &[]);
        let rt = ApiRuntime::new(client_with(two_turn_transport()), opts).expect("runtime");
        let mut stream = rt.run("add 2 and 3".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        assert!(!ran.load(Ordering::SeqCst), "denied tool must not execute");
        match frames.last().expect("terminal") {
            Message::Result(r) => {
                assert!(
                    !r.is_error,
                    "non-interrupt deny continues to a normal end_turn"
                );
                assert_eq!(r.stop_reason.as_deref(), Some("end_turn"));
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn dont_ask_allows_pre_approved_tool() {
        let ran = std::sync::Arc::new(AtomicBool::new(false));
        let opts = ran_flag_options(ran.clone(), PermissionMode::DontAsk, &["mcp__calc__add"]);
        let rt = ApiRuntime::new(client_with(two_turn_transport()), opts).expect("runtime");
        let mut stream = rt.run("add 2 and 3".into()).await.expect("run");
        let _ = collect(&mut stream).await;
        assert!(ran.load(Ordering::SeqCst), "pre-approved tool executes");
    }

    struct FixedJudge {
        decision: fn() -> PermissionDecision,
    }

    #[async_trait]
    impl PermissionJudge for FixedJudge {
        async fn judge(&self, _req: &JudgeRequest<'_>) -> Result<PermissionDecision, AgentErr> {
            Ok((self.decision)())
        }
    }

    fn auto_options_with_judge(
        ran: std::sync::Arc<AtomicBool>,
        judge: std::sync::Arc<dyn PermissionJudge>,
    ) -> Options {
        let add = tool(
            "add",
            "Add two ints",
            serde_json::json!({"type": "object"}),
            move |_args| {
                let ran_for_tool = ran.clone();
                async move {
                    ran_for_tool.store(true, Ordering::SeqCst);
                    Ok(ToolResult::text("5"))
                }
            },
        );
        Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .permission_mode(PermissionMode::Auto)
            .permission_judge(judge)
            .sdk_mcp_server(SdkMcpServer::builder("calc").tool(add).build())
            .build()
    }

    #[tokio::test]
    async fn auto_mode_judge_deny_blocks_the_tool() {
        let ran = std::sync::Arc::new(AtomicBool::new(false));
        let judge: std::sync::Arc<dyn PermissionJudge> = std::sync::Arc::new(FixedJudge {
            decision: || PermissionDecision::deny("no"),
        });
        let opts = auto_options_with_judge(ran.clone(), judge);
        let rt = ApiRuntime::new(client_with(two_turn_transport()), opts).expect("runtime");
        let mut stream = rt.run("add 2 and 3".into()).await.expect("run");
        let _ = collect(&mut stream).await;
        assert!(
            !ran.load(Ordering::SeqCst),
            "judge-denied tool must not execute"
        );
    }

    #[tokio::test]
    async fn auto_mode_judge_allow_runs_the_tool() {
        let ran = std::sync::Arc::new(AtomicBool::new(false));
        let judge: std::sync::Arc<dyn PermissionJudge> = std::sync::Arc::new(FixedJudge {
            decision: PermissionDecision::allow,
        });
        let opts = auto_options_with_judge(ran.clone(), judge);
        let rt = ApiRuntime::new(client_with(two_turn_transport()), opts).expect("runtime");
        let mut stream = rt.run("add 2 and 3".into()).await.expect("run");
        let _ = collect(&mut stream).await;
        assert!(ran.load(Ordering::SeqCst), "judge-allowed tool executes");
    }

    struct DenyInterruptPolicy;

    #[async_trait]
    impl PermissionPolicy for DenyInterruptPolicy {
        async fn can_use_tool(
            &self,
            _tool: &str,
            _input: &serde_json::Value,
            _ctx: PermissionContext,
        ) -> Result<PermissionDecision, AgentErr> {
            Ok(PermissionDecision::deny_interrupt("blocked"))
        }
    }

    #[tokio::test]
    async fn deny_interrupt_aborts_the_turn_with_permission_denied() {
        let mut transport = MockHttpTransport::new();
        // Only ONE request is issued: the loop aborts on the interrupt before the 2nd turn.
        transport
            .expect_send()
            .times(1)
            .returning(ok_response(TOOL_TURN));
        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .permission_policy(std::sync::Arc::new(DenyInterruptPolicy))
            .sdk_mcp_server(
                SdkMcpServer::builder("calc")
                    .tool(tool(
                        "add",
                        "d",
                        serde_json::json!({"type":"object"}),
                        |_| async { Ok(ToolResult::text("5")) },
                    ))
                    .build(),
            )
            .build();
        let rt = ApiRuntime::new(client_with(transport), opts).expect("runtime");
        let mut stream = rt.run("add".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => {
                assert!(r.is_error);
                assert_eq!(r.stop_reason.as_deref(), Some("permission_denied"));
                assert_eq!(r.result, "blocked");
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn turn_cap_exhaustion_yields_error_result() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().returning(ok_response(TOOL_TURN)); // always tool_use
        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .max_turns(2)
            .sdk_mcp_server(
                SdkMcpServer::builder("calc")
                    .tool(tool(
                        "add",
                        "d",
                        serde_json::json!({"type":"object"}),
                        |_| async { Ok(ToolResult::text("5")) },
                    ))
                    .build(),
            )
            .build();
        let rt = ApiRuntime::new(client_with(transport), opts).expect("runtime");
        let mut stream = rt.run("loop".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => {
                assert!(r.is_error);
                assert_eq!(r.stop_reason.as_deref(), Some("max_turns"));
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn wire_error_surfaces_on_the_stream() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body(
                r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#,
            ));
            *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            Ok(resp)
        });
        let rt = ApiRuntime::new(client_with(transport), options_with_model()).expect("runtime");
        let mut stream = rt.run("hi".into()).await.expect("run");
        let err = loop {
            match stream.next().await {
                Some(Err(e)) => break e,
                Some(Ok(_)) => {}
                None => panic!("expected an error frame"),
            }
        };
        assert!(matches!(err, AgentError::Protocol { .. }));
    }

    async fn collect(stream: &mut crate::agent::stream::MessageStream) -> Vec<Message> {
        let mut out = Vec::new();
        while let Some(item) = stream.next().await {
            out.push(item.expect("frame ok"));
        }
        out
    }

    #[test]
    fn model_reports_construction_identity() {
        let rt = ApiRuntime::new(client_with(MockHttpTransport::new()), options_with_model())
            .expect("runtime");
        assert_eq!(rt.model(), Some(&ModelId::claude_sonnet_4_5()));
    }

    use crate::agent::runtime::api::CachePolicy;

    fn capturing_transport(sink: std::sync::Arc<std::sync::Mutex<Vec<u8>>>) -> MockHttpTransport {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(move |req| {
            *sink.lock().unwrap() = req.body().to_vec();
            let mut resp = Response::new(body(END_TURN));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });
        transport
    }

    #[tokio::test]
    async fn prefix_policy_marks_the_request_with_cache_control() {
        let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let rt = ApiRuntime::new(
            client_with(capturing_transport(sink.clone())),
            calc_options(),
        )
        .expect("runtime")
        .with_cache_policy(CachePolicy::Prefix);
        let mut stream = rt.run("hi".into()).await.expect("run");
        let _ = collect(&mut stream).await;
        let sent = String::from_utf8(sink.lock().unwrap().clone()).expect("utf8");
        assert!(
            sent.contains("cache_control"),
            "request should carry a breakpoint: {sent}"
        );
    }

    #[tokio::test]
    async fn off_policy_leaves_the_request_uncached() {
        let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let rt = ApiRuntime::new(
            client_with(capturing_transport(sink.clone())),
            calc_options(),
        )
        .expect("runtime")
        .with_cache_policy(CachePolicy::Off);
        let mut stream = rt.run("hi".into()).await.expect("run");
        let _ = collect(&mut stream).await;
        let sent = String::from_utf8(sink.lock().unwrap().clone()).expect("utf8");
        assert!(
            !sent.contains("cache_control"),
            "Off must not cache: {sent}"
        );
    }

    const CACHE_TOOL_TURN: &str = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"tool_use","id":"toolu_1","name":"mcp__calc__add","input":{"a":2,"b":3}}],"stop_reason":"tool_use","stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":2,"cache_creation_input_tokens":100}}"#;
    const CACHE_FINAL_TURN: &str = r#"{"id":"msg_2","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"5"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":1,"cache_read_input_tokens":100}}"#;

    const END_TURN_JSON: &str = r#"{"id":"msg_2","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"{\"city\":\"Paris\"}"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":1}}"#;

    #[tokio::test]
    async fn structured_output_parsed_into_terminal_result() {
        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(1)
            .returning(ok_response(END_TURN_JSON));
        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .output_schema(serde_json::json!({ "type": "object" }))
            .build();
        let rt = ApiRuntime::new(client_with(transport), opts).expect("runtime");
        let mut stream = rt.run("hi".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => {
                assert_eq!(
                    r.structured_output,
                    Some(serde_json::json!({ "city": "Paris" }))
                );
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn no_output_format_leaves_structured_output_none() {
        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(1)
            .returning(ok_response(END_TURN));
        let rt = ApiRuntime::new(client_with(transport), options_with_model()).expect("runtime");
        let mut stream = rt.run("hi".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => assert!(r.structured_output.is_none()),
            other => panic!("expected Result, got {other:?}"),
        }
    }

    fn turn_ctx_with_output_format(
        output_format: Option<crate::messages::structured_outputs::OutputConfig>,
    ) -> super::TurnContext<MockHttpTransport> {
        use crate::agent::mcp::SdkMcpRegistry;
        use crate::agent::types::SessionId;

        super::TurnContext {
            client: client_with(MockHttpTransport::new()),
            registry: SdkMcpRegistry::default(),
            model: ModelId::claude_sonnet_4_5(),
            max_tokens: MaxTokens::new(64).unwrap(),
            system: None,
            turn_cap: 1,
            session_id: SessionId::new("test-session"),
            interrupt: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cache_policy: CachePolicy::default(),
            permission_mode: crate::agent::permissions::PermissionMode::default(),
            permission_policy: None,
            permission_judge: None,
            allowed_tools: Vec::new(),
            output_format,
            agents: std::collections::HashMap::new(),
            tool_filter: super::ToolFilter::inherit_all(),
            prior_history: Vec::new(),
            session_sink: None,
        }
    }

    #[test]
    fn build_request_attaches_output_config_when_output_format_is_some() {
        use crate::messages::structured_outputs::OutputConfig;

        let ctx = turn_ctx_with_output_format(Some(OutputConfig::json_schema(
            serde_json::json!({ "type": "object" }),
        )));
        let request = super::build_request(&ctx, &[], &[]);
        let json = serde_json::to_value(&request).expect("serialize");
        assert_eq!(
            json["output_config"]["format"]["type"], "json_schema",
            "build_request must attach output_config when output_format is Some: {json}"
        );
    }

    #[test]
    fn build_request_omits_output_config_when_output_format_is_none() {
        let ctx = turn_ctx_with_output_format(None);
        let request = super::build_request(&ctx, &[], &[]);
        let json = serde_json::to_value(&request).expect("serialize");
        assert!(
            json.get("output_config").is_none(),
            "build_request must omit output_config when output_format is None: {json}"
        );
    }

    #[tokio::test]
    async fn terminal_usage_sums_across_tool_loop_turns() {
        let mut transport = MockHttpTransport::new();
        let mut turn = 0u8;
        transport.expect_send().times(2).returning(move |_req| {
            turn += 1;
            let payload = if turn == 1 {
                CACHE_TOOL_TURN
            } else {
                CACHE_FINAL_TURN
            };
            let mut resp = Response::new(body(payload));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });
        let rt = ApiRuntime::new(client_with(transport), calc_options()).expect("runtime");
        let mut stream = rt.run("add 2 and 3".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => {
                let usage = r.usage.as_ref().expect("usage");
                assert_eq!(usage.input_tokens, 13);
                assert_eq!(usage.output_tokens, 3);
                assert_eq!(usage.cache_creation_input_tokens, Some(100));
                assert_eq!(usage.cache_read_input_tokens, Some(100));
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn second_run_sees_first_query_context() {
        let dir = tempfile::tempdir().expect("tempdir");
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(2)
            .returning(record_request_models(
                requests.clone(),
                vec![END_TURN, END_TURN],
            ));
        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .session_dir(dir.path().to_path_buf())
            .build();
        let rt = ApiRuntime::new(client_with(transport), opts).expect("runtime");

        let mut first = rt.run("first".into()).await.expect("run1");
        let _ = collect(&mut first).await;
        let mut second = rt.run("second".into()).await.expect("run2");
        let _ = collect(&mut second).await;

        let reqs = requests.lock().unwrap().clone();
        assert_eq!(reqs.len(), 2, "two requests, one per run");
        let second_body = reqs[1].to_string();
        assert!(
            second_body.contains("first"),
            "second request must carry the first query's turn: {second_body}"
        );
        assert!(
            second_body.contains("second"),
            "second request must carry its own prompt: {second_body}"
        );
    }

    #[test]
    fn resolve_new_mints_matching_load_and_write() {
        use crate::agent::types::SessionControl;
        let dir = tempfile::tempdir().expect("tempdir");
        let store = super::SessionStore::new(dir.path().to_path_buf());
        let ids = super::resolve_session(&SessionControl::New, &store).expect("resolve");
        assert_eq!(ids.load, ids.write);
    }

    #[test]
    fn resolve_continue_empty_store_mints() {
        use crate::agent::types::SessionControl;
        let dir = tempfile::tempdir().expect("tempdir");
        let store = super::SessionStore::new(dir.path().to_path_buf());
        let ids = super::resolve_session(&SessionControl::Continue { fork: false }, &store)
            .expect("resolve");
        assert_eq!(ids.load, ids.write);
    }

    #[tokio::test]
    async fn resume_unknown_id_fails_construction() {
        use crate::agent::types::SessionId;
        let dir = tempfile::tempdir().expect("tempdir");
        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .session_dir(dir.path().to_path_buf())
            .session(crate::agent::types::SessionControl::Resume {
                id: SessionId::new("ghost"),
                fork: false,
            })
            .build();
        let result = ApiRuntime::new(client_with(MockHttpTransport::new()), opts);
        assert!(result.is_err(), "resuming an unknown id must fail closed");
    }

    #[test]
    fn resolve_resume_existing_ok_and_fork_branches() {
        use crate::agent::types::{SessionControl, SessionId};
        let dir = tempfile::tempdir().expect("tempdir");
        let store = super::SessionStore::new(dir.path().to_path_buf());
        let id = SessionId::new("sess_live");
        store.save(&id, &[]).expect("seed");

        let ids = super::resolve_session(
            &SessionControl::Resume {
                id: id.clone(),
                fork: false,
            },
            &store,
        )
        .expect("resolve");
        assert_eq!(ids.load, id);
        assert_eq!(ids.write, id);

        let forked = super::resolve_session(
            &SessionControl::Resume {
                id: id.clone(),
                fork: true,
            },
            &store,
        )
        .expect("resolve");
        assert_eq!(forked.load, id);
        assert_ne!(forked.write, id, "fork writes a new id");
    }

    #[tokio::test]
    async fn fork_leaves_source_file_untouched() {
        use crate::agent::types::{SessionControl, SessionId};
        use crate::messages::request::{InputMessage, MessageContent, Role};

        let dir = tempfile::tempdir().expect("tempdir");
        let store = super::SessionStore::new(dir.path().to_path_buf());
        let source = SessionId::new("sess_source");
        store
            .save(
                &source,
                &[InputMessage {
                    role: Role::User,
                    content: MessageContent::Text("orig".to_string()),
                }],
            )
            .expect("seed source");
        let before = std::fs::read(dir.path().join("sess_source.json")).expect("read before");

        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(1)
            .returning(ok_response(END_TURN));
        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .session_dir(dir.path().to_path_buf())
            .session(SessionControl::Resume {
                id: source.clone(),
                fork: true,
            })
            .build();
        let rt = ApiRuntime::new(client_with(transport), opts).expect("runtime");
        let mut stream = rt.run("more".into()).await.expect("run");
        let _ = collect(&mut stream).await;

        let after = std::fs::read(dir.path().join("sess_source.json")).expect("read after");
        assert_eq!(
            before, after,
            "fork must not modify the source session file"
        );

        let json_files = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .count();
        assert!(
            json_files >= 2,
            "fork writes a new session file alongside the source"
        );
    }

    #[tokio::test]
    async fn fork_first_run_failure_keeps_source_reseedable() {
        use crate::agent::types::{SessionControl, SessionId};
        use crate::messages::request::{InputMessage, MessageContent, Role};

        let dir = tempfile::tempdir().expect("tempdir");
        let store = super::SessionStore::new(dir.path().to_path_buf());
        let source = SessionId::new("sess_src");
        store
            .save(
                &source,
                &[InputMessage {
                    role: Role::User,
                    content: MessageContent::Text("orig".to_string()),
                }],
            )
            .expect("seed source");

        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(2).returning({
            let requests = requests.clone();
            let calls = calls.clone();
            move |req| {
                let parsed: serde_json::Value =
                    serde_json::from_slice(req.body()).unwrap_or(serde_json::Value::Null);
                requests.lock().unwrap().push(parsed);
                let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n == 0 {
                    Err(TransportError::Network("boom".to_string()))
                } else {
                    let mut resp = Response::new(body(END_TURN));
                    *resp.status_mut() = StatusCode::OK;
                    Ok(resp)
                }
            }
        });

        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .session_dir(dir.path().to_path_buf())
            .session(SessionControl::Resume {
                id: source.clone(),
                fork: true,
            })
            .build();
        let rt = ApiRuntime::new(client_with(transport), opts).expect("runtime");

        // The first run's transport failure surfaces as an error frame, not a
        // panic-worthy `Ok` frame, so drain it directly rather than via
        // `collect` (which asserts every frame is `Ok`).
        let mut first = rt.run("attempt1".into()).await.expect("run1");
        while first.next().await.is_some() {}
        let mut second = rt.run("attempt2".into()).await.expect("run2");
        let _ = collect(&mut second).await;

        let reqs = requests.lock().unwrap().clone();
        assert_eq!(reqs.len(), 2, "one request per run");
        let second_body = reqs[1].to_string();
        assert!(
            second_body.contains("orig"),
            "second request must still carry the source session's history \
             after the first run failed before persist: {second_body}"
        );
        assert!(
            second_body.contains("attempt2"),
            "second request must carry its own prompt: {second_body}"
        );
    }

    #[test]
    fn list_sessions_reports_stored_sessions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let opts = Options::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(64).unwrap())
            .session_dir(dir.path().to_path_buf())
            .build();
        let rt = ApiRuntime::new(client_with(MockHttpTransport::new()), opts).expect("runtime");
        assert!(rt.list_sessions().expect("list").is_empty());

        std::fs::create_dir_all(dir.path()).expect("mkdir");
        std::fs::write(dir.path().join("sess_a.json"), b"[]").expect("write");
        let ids: Vec<String> = rt
            .list_sessions()
            .expect("list")
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        assert_eq!(ids, vec!["sess_a".to_string()]);
    }
}
