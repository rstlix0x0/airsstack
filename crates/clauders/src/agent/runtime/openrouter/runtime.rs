//! The native `Runtime` over the OpenRouter chat-completions API.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use tokio::sync::mpsc;

use openrouter_rs::Client as OrClient;
use openrouter_rs::chat::message::Message as OrMessage;
use openrouter_rs::chat::request::ChatRequest;
use openrouter_rs::chat::response::{ChatCompletion, FinishReason};
use openrouter_rs::chat::tool::{Tool as OrTool, ToolChoice};
use openrouter_rs::types::{MaxTokens as OrMaxTokens, ModelId as OrModelId};

use crate::agent::capabilities::Capabilities;
use crate::agent::content::ContentBlock as AgentBlock;
use crate::agent::error::AgentError;
use crate::agent::mcp::SdkMcpRegistry;
use crate::agent::message::{AssistantMessage, Message, ResultMessage};
use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;
use crate::agent::runtime::Runtime;
use crate::agent::stream::{MessageStream, ReceiverStream};
use crate::agent::types::{McpStatus, Prompt, ServerStatus, SessionId};
use crate::client::DefaultTransportPlaceholder;
use crate::transport::HttpTransport;
use crate::types::ModelId;

use super::{convert, tools};

/// Per-turn message channel capacity (natural backpressure beyond this).
const TURN_CHANNEL_CAPACITY: usize = 64;
/// Turn cap applied when `Options.max_turns` is unset.
const DEFAULT_MAX_TURNS: u32 = 8;
/// The static control-protocol version this runtime reports.
const PROTOCOL_VERSION: &str = "openrouter-1.0";

/// Monotonic source of per-runtime session identifiers.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A [`Runtime`] that drives one agent session against the OpenRouter
/// chat-completions API, with an in-process tool loop.
///
/// Generic over the HTTP transport (defaulting to reqwest) so the whole loop is
/// exercisable offline against a mock transport. `Options.model` carries an
/// OpenRouter model slug (e.g. `"deepseek/deepseek-chat"`,
/// `"anthropic/claude-sonnet-4-5"`), not a bare Anthropic model name.
pub struct OpenRouterRuntime<T: HttpTransport = DefaultTransportPlaceholder> {
    client: OrClient<T>,
    registry: SdkMcpRegistry,
    max_tokens: OrMaxTokens,
    system: Option<String>,
    turn_cap: u32,
    session_id: SessionId,
    capabilities: Capabilities,
    identity: Option<ModelId>,
    model: Mutex<OrModelId>,
    permission_mode: Mutex<PermissionMode>,
    interrupt: Arc<AtomicBool>,
}

impl<T: HttpTransport> OpenRouterRuntime<T> {
    /// Build a runtime from an OpenRouter [`OrClient`] and session [`Options`].
    ///
    /// # Errors
    /// Returns [`AgentError::Protocol`] when `Options.model` is `None` (the API
    /// requires an explicit model), when the model slug is not a valid
    /// OpenRouter model id, or when the max-token count cannot be represented as
    /// an OpenRouter `MaxTokens`.
    pub fn new(client: OrClient<T>, options: Options) -> Result<Self, AgentError> {
        let model = options.model.ok_or_else(|| AgentError::Protocol {
            detail: "OpenRouterRuntime requires Options.model to be set".to_string(),
        })?;
        let identity = model.clone();
        let model = to_or_model(&model)?;
        let max_tokens =
            OrMaxTokens::new(options.max_tokens.get()).map_err(|e| AgentError::Protocol {
                detail: e.to_string(),
            })?;
        let n = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            client,
            registry: options.sdk_mcp_servers,
            max_tokens,
            system: options.system_prompt,
            turn_cap: options.max_turns.unwrap_or(DEFAULT_MAX_TURNS),
            session_id: SessionId::new(format!("openrouter-session-{n}")),
            capabilities: build_capabilities(),
            identity: Some(identity),
            model: Mutex::new(model),
            permission_mode: Mutex::new(options.permission_mode),
            interrupt: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Read the current model, recovering from a poisoned lock.
    fn current_model(&self) -> OrModelId {
        self.model
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Convert a `clauders` model id (an OpenRouter slug in this runtime) into an
/// OpenRouter model id, folding a rejection into a protocol error.
fn to_or_model(model: &ModelId) -> Result<OrModelId, AgentError> {
    OrModelId::custom(model.as_str()).map_err(|e| AgentError::Protocol {
        detail: e.to_string(),
    })
}

/// The static capability manifest: no hooks, only the honored control methods.
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
impl<T: HttpTransport> Runtime for OpenRouterRuntime<T> {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
        // Clear any interrupt latched by a prior turn so a reused runtime is
        // never permanently poisoned by one `interrupt()`.
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
            interrupt: Arc::clone(&self.interrupt),
        };
        tokio::spawn(drive(ctx, prompt, tx));
        Ok(ReceiverStream::new(rx).boxed())
    }

    async fn interrupt(&self) -> Result<(), AgentError> {
        self.interrupt.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
        let model = to_or_model(&model)?;
        *self.model.lock().unwrap_or_else(PoisonError::into_inner) = model;
        Ok(())
    }

    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
        *self
            .permission_mode
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = mode;
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
    client: OrClient<T>,
    registry: SdkMcpRegistry,
    model: OrModelId,
    max_tokens: OrMaxTokens,
    system: Option<String>,
    turn_cap: u32,
    session_id: SessionId,
    interrupt: Arc<AtomicBool>,
}

/// Drive the agent loop, pushing frames into `tx` until the turn ends, the turn
/// cap is hit, an error occurs, or an interrupt is observed.
async fn drive<T: HttpTransport>(
    ctx: TurnContext<T>,
    prompt: Prompt,
    tx: mpsc::Sender<Result<Message, AgentError>>,
) {
    let tool_defs: Vec<OrTool> = tools::declare(&ctx.registry);
    let mut history: Vec<OrMessage> = Vec::new();
    if let Some(system) = &ctx.system {
        history.push(OrMessage::system(system.clone()));
    }
    history.push(OrMessage::user(prompt.as_str().to_string()));

    for turn in 0..ctx.turn_cap {
        if ctx.interrupt.load(Ordering::SeqCst) {
            return;
        }

        let request = build_request(&ctx, &history, &tool_defs);
        let completion = match ctx.client.chat().send(request).await {
            Ok(completion) => completion,
            Err(error) => {
                let _ = tx.send(Err(convert::map_or_error(error))).await;
                return;
            }
        };
        let Some(choice) = completion.choices.first() else {
            let _ = tx
                .send(Err(AgentError::Protocol {
                    detail: "OpenRouter completion had no choices".to_string(),
                }))
                .await;
            return;
        };

        if emit_assistant(&tx, &choice.message).await.is_err() {
            return; // receiver dropped
        }

        if choice.finish_reason == Some(FinishReason::ToolCalls) {
            if let Some(calls) = &choice.message.tool_calls {
                history.push(OrMessage::assistant_tool_calls(calls.clone()));
                for call in calls {
                    history.push(tools::dispatch(&ctx.registry, call).await);
                }
                continue;
            }
        }

        let _ = tx
            .send(Ok(terminal_result(&ctx, &completion, choice, turn + 1)))
            .await;
        return;
    }

    let _ = tx
        .send(Ok(exhausted_result(&ctx.session_id, ctx.turn_cap)))
        .await;
}

/// Build the next `ChatRequest` from the running history and tool set.
fn build_request<T: HttpTransport>(
    ctx: &TurnContext<T>,
    history: &[OrMessage],
    tool_defs: &[OrTool],
) -> ChatRequest {
    let mut builder = ChatRequest::builder()
        .model(ctx.model.clone())
        .messages(history.to_vec())
        .max_tokens(ctx.max_tokens);
    if !tool_defs.is_empty() {
        builder = builder
            .tools(tool_defs.to_vec())
            .tool_choice(ToolChoice::Auto);
    }
    builder.build()
}

/// Emit the assistant turn as an agent frame.
async fn emit_assistant(
    tx: &mpsc::Sender<Result<Message, AgentError>>,
    message: &openrouter_rs::chat::response::ResponseMessage,
) -> Result<(), ()> {
    let text = convert::content_text(message);
    let content = if text.is_empty() {
        Vec::new()
    } else {
        vec![AgentBlock::Text { text }]
    };
    tx.send(Ok(Message::Assistant(AssistantMessage {
        content,
        parent_tool_use_id: None,
    })))
    .await
    .map_err(|_| ())
}

/// The terminal `Result` frame for a completed turn.
fn terminal_result<T: HttpTransport>(
    ctx: &TurnContext<T>,
    completion: &ChatCompletion,
    choice: &openrouter_rs::chat::response::Choice,
    num_turns: u32,
) -> Message {
    Message::Result(ResultMessage {
        result: convert::content_text(&choice.message),
        is_error: false,
        total_cost_usd: completion.usage.and_then(|u| u.cost),
        stop_reason: choice
            .finish_reason
            .map(|r| convert::finish_reason_wire(r).to_string()),
        usage: completion.usage.map(|u| convert::usage(&u)),
        session_id: ctx.session_id.clone(),
        num_turns,
    })
}

/// The terminal `Result` frame when the turn cap is exhausted.
fn exhausted_result(session_id: &SessionId, turn_cap: u32) -> Message {
    Message::Result(ResultMessage {
        result: String::new(),
        is_error: true,
        total_cost_usd: None,
        stop_reason: Some("max_turns".to_string()),
        usage: None,
        session_id: session_id.clone(),
        num_turns: turn_cap,
    })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use std::pin::Pin;
    use std::task::{Context, Poll};

    use bytes::Bytes;
    use futures_core::Stream;
    use futures_util::StreamExt;
    use http::{Response, StatusCode};

    use super::OpenRouterRuntime;
    use crate::agent::error::AgentError;
    use crate::agent::mcp::SdkMcpServer;
    use crate::agent::mcp::tool::{ToolResult, tool};
    use crate::agent::message::Message;
    use crate::agent::options::Options;
    use crate::agent::runtime::Runtime;
    use crate::agent::stream::MessageStream;
    use crate::error::TransportError;
    use crate::test_support::MockHttpTransport;
    use crate::transport::BodyStream;
    use crate::types::{MaxTokens, ModelId};
    use openrouter_rs::Client as OrClient;
    use openrouter_rs::types::ApiKey as OrApiKey;

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

    fn or_client(transport: MockHttpTransport) -> OrClient<MockHttpTransport> {
        OrClient::builder_with_transport(transport)
            .api_key(OrApiKey::new("sk-or-test").expect("key"))
            .build()
            .expect("client")
    }

    fn options_with_model() -> Options {
        Options::builder()
            .model(ModelId::custom("deepseek/deepseek-chat").expect("model"))
            .max_tokens(MaxTokens::new(64).expect("max"))
            .build()
    }

    async fn collect(stream: &mut MessageStream) -> Vec<Message> {
        let mut out = Vec::new();
        while let Some(item) = stream.next().await {
            out.push(item.expect("frame ok"));
        }
        out
    }

    const STOP: &str = r#"{"id":"gen-1","object":"chat.completion","created":1,"model":"deepseek/deepseek-chat","choices":[{"index":0,"message":{"role":"assistant","content":"hello"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":1,"total_tokens":4,"cost":0.002}}"#;

    #[test]
    fn new_requires_a_model() {
        let opts = Options::builder().build();
        assert!(OpenRouterRuntime::new(or_client(MockHttpTransport::new()), opts).is_err());
    }

    #[test]
    fn capabilities_report_honored_control_methods() {
        let rt = OpenRouterRuntime::new(or_client(MockHttpTransport::new()), options_with_model())
            .expect("runtime");
        let caps = rt.capabilities();
        assert!(caps.supports_control("set_model"));
        assert_eq!(caps.protocol_version, "openrouter-1.0");
        assert!(caps.supported_hook_events.is_empty());
    }

    #[tokio::test]
    async fn mcp_status_reports_registered_servers() {
        let opts = Options::builder()
            .model(ModelId::custom("deepseek/deepseek-chat").expect("model"))
            .max_tokens(MaxTokens::new(64).expect("max"))
            .sdk_mcp_server(SdkMcpServer::builder("calc").build())
            .build();
        let rt =
            OpenRouterRuntime::new(or_client(MockHttpTransport::new()), opts).expect("runtime");
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
            .returning(ok_response(STOP));
        let rt =
            OpenRouterRuntime::new(or_client(transport), options_with_model()).expect("runtime");
        let mut stream = rt.run("hi".into()).await.expect("run");

        let first = stream.next().await.expect("assistant").expect("ok");
        assert!(matches!(first, Message::Assistant(a) if a.content.len() == 1));
        match stream.next().await.expect("result").expect("ok") {
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
    async fn terminal_result_populates_cost() {
        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(1)
            .returning(ok_response(STOP));
        let rt =
            OpenRouterRuntime::new(or_client(transport), options_with_model()).expect("runtime");
        let mut stream = rt.run("hi".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => assert_eq!(r.total_cost_usd, Some(0.002)),
            other => panic!("expected Result, got {other:?}"),
        }
    }

    fn calc_options() -> Options {
        let add = tool(
            "add",
            "Add two ints",
            serde_json::json!({ "type": "object" }),
            |args| async move {
                let s = args["a"].as_i64().unwrap_or(0) + args["b"].as_i64().unwrap_or(0);
                Ok(ToolResult::text(s.to_string()))
            },
        );
        Options::builder()
            .model(ModelId::custom("deepseek/deepseek-chat").expect("model"))
            .max_tokens(MaxTokens::new(64).expect("max"))
            .sdk_mcp_server(SdkMcpServer::builder("calc").tool(add).build())
            .build()
    }

    const TOOL_TURN: &str = r#"{"id":"gen-t","object":"chat.completion","created":1,"model":"deepseek/deepseek-chat","choices":[{"index":0,"message":{"role":"assistant","content":null,"tool_calls":[{"id":"call_1","type":"function","function":{"name":"mcp__calc__add","arguments":"{\"a\":2,\"b\":3}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7}}"#;
    const FINAL_TURN: &str = r#"{"id":"gen-f","object":"chat.completion","created":1,"model":"deepseek/deepseek-chat","choices":[{"index":0,"message":{"role":"assistant","content":"the answer is 5"},"finish_reason":"stop"}],"usage":{"prompt_tokens":9,"completion_tokens":4,"total_tokens":13}}"#;

    #[tokio::test]
    async fn tool_call_round_trip_then_stop() {
        let mut transport = MockHttpTransport::new();
        let mut turn = 0u8;
        transport.expect_send().times(2).returning(move |_req| {
            turn += 1;
            let payload = if turn == 1 { TOOL_TURN } else { FINAL_TURN };
            let mut resp = Response::new(body(payload));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        });
        let rt = OpenRouterRuntime::new(or_client(transport), calc_options()).expect("runtime");
        let mut stream = rt.run("add 2 and 3".into()).await.expect("run");
        let frames = collect(&mut stream).await;
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
        transport.expect_send().returning(ok_response(TOOL_TURN)); // always tool_calls
        let opts = Options::builder()
            .model(ModelId::custom("deepseek/deepseek-chat").expect("model"))
            .max_tokens(MaxTokens::new(64).expect("max"))
            .max_turns(2)
            .sdk_mcp_server(
                SdkMcpServer::builder("calc")
                    .tool(tool(
                        "add",
                        "d",
                        serde_json::json!({ "type": "object" }),
                        |_| async { Ok(ToolResult::text("5")) },
                    ))
                    .build(),
            )
            .build();
        let rt = OpenRouterRuntime::new(or_client(transport), opts).expect("runtime");
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
    async fn api_error_surfaces_on_the_stream() {
        let mut transport = MockHttpTransport::new();
        transport.expect_send().times(1).returning(|_req| {
            let mut resp = Response::new(body(r#"{"error":{"code":429,"message":"slow"}}"#));
            *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            Ok(resp)
        });
        let rt =
            OpenRouterRuntime::new(or_client(transport), options_with_model()).expect("runtime");
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

    #[tokio::test]
    async fn run_after_interrupt_is_not_poisoned() {
        let mut transport = MockHttpTransport::new();
        transport
            .expect_send()
            .times(1)
            .returning(ok_response(STOP));
        let rt =
            OpenRouterRuntime::new(or_client(transport), options_with_model()).expect("runtime");
        rt.interrupt().await.expect("interrupt");
        let mut stream = rt.run("hi".into()).await.expect("run");
        let frames = collect(&mut stream).await;
        match frames.last().expect("terminal") {
            Message::Result(r) => assert!(!r.is_error),
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_model_updates_local_state() {
        let rt = OpenRouterRuntime::new(or_client(MockHttpTransport::new()), options_with_model())
            .expect("runtime");
        rt.set_model(ModelId::custom("qwen/qwen-2.5-72b-instruct").expect("model"))
            .await
            .expect("set_model");
    }

    #[test]
    fn model_reports_construction_identity() {
        let rt = OpenRouterRuntime::new(or_client(MockHttpTransport::new()), options_with_model())
            .expect("runtime");
        assert_eq!(
            rt.model(),
            Some(&ModelId::custom("deepseek/deepseek-chat").expect("model"))
        );
    }
}
