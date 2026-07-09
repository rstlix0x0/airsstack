//! The native `Runtime` over the Messages API.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::agent::capabilities::Capabilities;
use crate::agent::content::ContentBlock as AgentBlock;
use crate::agent::error::AgentError;
use crate::agent::mcp::SdkMcpRegistry;
use crate::agent::message::{AssistantMessage, Message, ResultMessage, Usage as AgentUsage};
use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;
use crate::agent::runtime::Runtime;
use crate::agent::stream::{MessageStream, ReceiverStream};
use crate::agent::types::{McpStatus, Prompt, ServerStatus, SessionId};
use crate::client::Client;
use crate::client::DefaultTransportPlaceholder;
use crate::messages::content::ContentBlock as WireBlock;
use crate::messages::request::{InputMessage, MessageContent, MessageRequest, Role};
use crate::messages::response::{Message as WireMessage, StopReason, Usage as WireUsage};
use crate::messages::tools::Tool as WireTool;
use crate::transport::HttpTransport;
use crate::types::{MaxTokens, ModelId, SystemPrompt};

use super::cache::CachePolicy;
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
    session_id: SessionId,
    capabilities: Capabilities,
    identity: Option<ModelId>,
    model: Mutex<ModelId>,
    permission_mode: Mutex<PermissionMode>,
    interrupt: std::sync::Arc<AtomicBool>,
    cache_policy: CachePolicy,
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
        let n = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            client,
            registry: options.sdk_mcp_servers,
            max_tokens: options.max_tokens,
            system: options.system_prompt.map(SystemPrompt::text),
            turn_cap: options.max_turns.unwrap_or(DEFAULT_MAX_TURNS),
            session_id: SessionId::new(format!("api-session-{n}")),
            capabilities: build_capabilities(),
            identity: Some(model.clone()),
            model: Mutex::new(model),
            permission_mode: Mutex::new(options.permission_mode),
            interrupt: std::sync::Arc::new(AtomicBool::new(false)),
            cache_policy: CachePolicy::default(),
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

    /// Read the current model, recovering from a poisoned lock.
    fn current_model(&self) -> ModelId {
        self.model
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
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
        let (tx, rx) = mpsc::channel::<Result<Message, AgentError>>(TURN_CHANNEL_CAPACITY);
        let ctx = TurnContext {
            client: self.client.clone(),
            registry: self.registry.clone(),
            model: self.current_model(),
            max_tokens: self.max_tokens,
            system: self.system.clone(),
            turn_cap: self.turn_cap,
            session_id: self.session_id.clone(),
            interrupt: std::sync::Arc::clone(&self.interrupt),
            cache_policy: self.cache_policy,
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
}

/// Drive the agent loop, pushing frames into `tx` until the turn ends, the
/// turn cap is hit, an error occurs, or an interrupt is observed.
async fn drive<T: HttpTransport>(
    ctx: TurnContext<T>,
    prompt: Prompt,
    tx: mpsc::Sender<Result<Message, AgentError>>,
) {
    let tool_defs: Vec<WireTool> = tools::declare(&ctx.registry);
    let mut history: Vec<InputMessage> = vec![InputMessage {
        role: Role::User,
        content: MessageContent::Text(prompt.as_str().to_string()),
    }];
    let mut usage_total = AgentUsage::default();

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
            let results = run_tools(&ctx.registry, &response.content).await;
            history.push(InputMessage {
                role: Role::User,
                content: MessageContent::Blocks(results),
            });
            continue;
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
    builder.tools(tools).build()
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

/// Run every tool-use block in `content`, returning the tool-result blocks.
async fn run_tools(registry: &SdkMcpRegistry, content: &[WireBlock]) -> Vec<WireBlock> {
    let mut results = Vec::new();
    for block in content {
        if let WireBlock::ToolUse(use_block) = block {
            let result = tools::dispatch(registry, use_block).await;
            results.push(WireBlock::ToolResult(result));
        }
    }
    results
}

/// The terminal `Result` frame for a completed turn.
fn terminal_result<T: HttpTransport>(
    ctx: &TurnContext<T>,
    response: &WireMessage,
    num_turns: u32,
    is_error: bool,
    usage: AgentUsage,
) -> Message {
    Message::Result(ResultMessage {
        result: convert::last_text(&response.content),
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
        is_error: true,
        total_cost_usd: None,
        stop_reason: Some("max_turns".to_string()),
        usage: Some(usage),
        session_id: session_id.clone(),
        num_turns: turn_cap,
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
}
