//! The subprocess-backed `Runtime` implementation.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_core::Stream;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::agent::capabilities::Capabilities;
use crate::agent::error::AgentError;
use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;
use crate::agent::process::{LineError, ManagedProcess, ProcessConfig, ProcessIo, StdoutLines};
use crate::agent::protocol::{
    ControlResponseBody, OutboundControlRequest, OutboundRequestBody, RequestId, RequestIdGen,
    decode_inbound, encode_line,
};
use crate::agent::runtime::Runtime;
use crate::agent::stream::{MessageStream, ReceiverStream};
use crate::agent::types::{McpStatus, Prompt};
use crate::types::ModelId;

use super::argv::{build_argv, permission_mode_wire};
use super::demux::Demux;
use super::discovery::{check_version, discover};
use super::dispatch::Dispatcher;
use super::handshake::{initialize_request, parse_capabilities, warn_unsupported_hooks};

/// Per-turn message channel capacity (natural backpressure beyond this).
const TURN_CHANNEL_CAPACITY: usize = 64;

/// A `Runtime` that drives the backend binary over its control protocol.
pub struct CliRuntime {
    // Outbound lines are funneled to a single writer task that owns stdin, so
    // run(), control requests, and reader-spawned dispatch never contend on it.
    out_tx: mpsc::UnboundedSender<String>,
    demux: Arc<Demux>,
    id_gen: RequestIdGen,
    capabilities: Capabilities,
    control_request_timeout: Duration,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
    _process: ManagedProcess,
}

impl CliRuntime {
    /// Discover, spawn, and handshake with the backend, returning a runtime.
    ///
    /// # Errors
    /// Returns an [`AgentError`] if the binary cannot be located or is too
    /// old (when required), if the process cannot be spawned, or if the
    /// initialize handshake does not complete.
    pub async fn connect(options: Options) -> Result<Self, AgentError> {
        let program = discover(&options)?;
        if let Some(reported) = probe_version(&program).await {
            check_version(&reported, options.require_min_version)?;
        }

        let cfg = ProcessConfig {
            program,
            args: build_argv(&options),
            cwd: options.cwd.clone(),
            env: options.env.clone(),
            shutdown_grace: options.shutdown_grace,
            max_buffer_size: options.max_buffer_size,
            stderr_callback: options.stderr.clone(),
        };
        let (process, io) = ManagedProcess::spawn(&cfg)?;
        let ProcessIo {
            stdin,
            stdout,
            stderr,
        } = io;
        let mut stdin = stdin;
        let mut stdout = stdout;

        let id_gen = RequestId::generator();
        let capabilities = handshake(&mut stdin, &mut stdout, &options, &id_gen).await?;
        warn_unsupported_hooks(&options, &capabilities);

        // Single writer task owns stdin from here on.
        let (out_tx, out_rx) = mpsc::unbounded_channel::<String>();
        let writer = tokio::spawn(writer_loop(stdin, out_rx));

        // Extract handlers for the dispatcher (Arc-shared; cheap clone).
        let hooks = Arc::new(options.hooks.clone());
        let policy = options.permission_policy.clone();
        let elicitation = options.elicitation_policy.clone();
        let mcp = Arc::new(options.sdk_mcp_servers.clone());
        let dispatcher = Arc::new(Dispatcher::new(
            hooks,
            policy,
            elicitation,
            mcp,
            out_tx.clone(),
        ));

        let demux = Arc::new(Demux::new());
        let reader = tokio::spawn(reader_loop(stdout, Arc::clone(&demux), dispatcher));
        // stderr is drained by the process layer; not needed for message routing.
        drop(stderr);

        Ok(Self {
            out_tx,
            demux,
            id_gen,
            capabilities,
            control_request_timeout: options.control_request_timeout,
            reader,
            writer,
            _process: process,
        })
    }

    /// Send a control request and await its correlated response payload.
    async fn send_control(
        &self,
        body: OutboundRequestBody,
        method: &str,
    ) -> Result<serde_json::Value, AgentError> {
        let id = self.id_gen.next();

        let request = OutboundControlRequest {
            kind: "control_request",
            request_id: id.as_str(),
            request: body,
        };
        // Encode before registering so an encode failure needs no cleanup.
        let line = encode_line(&request)?;

        // If this call is cancelled (e.g. dropped mid-`select!`) before any
        // of the paths below runs, this entry outlives it in `pending`. The
        // leak is bounded, not unbounded: `Demux::close()` drains every
        // remaining entry when the transport closes.
        //
        // If `close()` has already run, `register_pending` refuses to
        // register at all and fails fast with `TransportClosed` here,
        // rather than parking this waiter on a transport already known to
        // be dead until the full control-request timeout elapses.
        let (tx, rx) = oneshot::channel();
        self.demux.register_pending(id.as_str().to_string(), tx)?;

        if self.out_tx.send(line).is_err() {
            self.demux.remove_pending(id.as_str());
            return Err(AgentError::TransportClosed);
        }

        let result = await_control_response(rx, self.control_request_timeout, method).await;
        if matches!(result, Err(AgentError::ControlRequestTimedOut { .. })) {
            // The response may still arrive after the deadline; drop the
            // stale waiter so a late `control_response` cannot resolve a
            // receiver nobody is awaiting anymore, and so `pending` does not
            // grow unbounded across repeated timeouts.
            self.demux.remove_pending(id.as_str());
        }
        result
    }
}

/// Await one control response, bounded by `timeout`.
///
/// Three outcomes: the response arrives in time (resolved via
/// [`control_response_outcome`]); the sender is dropped without ever
/// responding — e.g. because the transport closed and drained every pending
/// waiter — which maps to [`AgentError::TransportClosed`]; or the bound
/// elapses first, which maps to [`AgentError::ControlRequestTimedOut`].
async fn await_control_response(
    rx: oneshot::Receiver<ControlResponseBody>,
    timeout: Duration,
    method: &str,
) -> Result<serde_json::Value, AgentError> {
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(response)) => control_response_outcome(response, method),
        Ok(Err(_)) => Err(AgentError::TransportClosed),
        Err(_elapsed) => Err(AgentError::ControlRequestTimedOut {
            method: method.to_string(),
            elapsed: timeout,
        }),
    }
}

/// Resolve a correlated control response into the caller-facing result.
///
/// `Malformed` — a response body that failed to deserialize but whose
/// `request_id` still correlated to this waiter (see
/// [`ControlResponseBody`]) — resolves to an error rather than leaving the
/// caller waiting: the outbound mirror of how an inbound `control_request`
/// that fails to deserialize still gets answered instead of hanging the
/// binary.
fn control_response_outcome(
    response: ControlResponseBody,
    method: &str,
) -> Result<serde_json::Value, AgentError> {
    match response {
        ControlResponseBody::Success { response, .. } => Ok(response),
        ControlResponseBody::Error { error, .. } => Err(AgentError::ControlRequestFailed {
            method: method.to_string(),
            detail: error,
        }),
        ControlResponseBody::Malformed { detail, .. } => Err(AgentError::ControlRequestFailed {
            method: method.to_string(),
            detail,
        }),
    }
}

impl Drop for CliRuntime {
    fn drop(&mut self) {
        // Stop both tasks; the process handle's own Drop tears the child down.
        self.reader.abort();
        self.writer.abort();
    }
}

#[async_trait]
impl Runtime for CliRuntime {
    async fn run(&self, prompt: Prompt) -> Result<MessageStream, AgentError> {
        let (tx, rx) = mpsc::channel(TURN_CHANNEL_CAPACITY);
        self.demux.set_turn_sink(tx);
        match prompt {
            Prompt::Single(text) => {
                let line = encode_line(&user_message_frame(&text))?;
                if self.out_tx.send(line).is_err() {
                    return Err(AgentError::TransportClosed);
                }
            }
            Prompt::Stream(stream) => {
                tokio::spawn(drain_input(stream, self.out_tx.clone()));
            }
        }
        Ok(ReceiverStream::new(rx).boxed())
    }

    async fn interrupt(&self) -> Result<(), AgentError> {
        self.send_control(OutboundRequestBody::Interrupt, "interrupt")
            .await
            .map(|_| ())
    }

    async fn set_model(&self, model: ModelId) -> Result<(), AgentError> {
        self.send_control(
            OutboundRequestBody::SetModel {
                model: model.as_str().to_string(),
            },
            "set_model",
        )
        .await
        .map(|_| ())
    }

    async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), AgentError> {
        self.send_control(
            OutboundRequestBody::SetPermissionMode {
                mode: permission_mode_wire(mode).to_string(),
            },
            "set_permission_mode",
        )
        .await
        .map(|_| ())
    }

    async fn mcp_status(&self) -> Result<McpStatus, AgentError> {
        let value = self
            .send_control(OutboundRequestBody::McpStatus, "mcp_status")
            .await?;
        serde_json::from_value(value).map_err(|e| AgentError::Decode(e.to_string()))
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
}

/// Build the outbound user-message frame carrying one prompt text.
fn user_message_frame(text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "user",
        "message": { "role": "user", "content": text }
    })
}

/// Feed a streamed prompt to stdin: one user-message line per item, as they
/// arrive. Ends when the stream is exhausted or the writer channel is gone.
/// Drives the stream with `poll_fn` + `Stream::poll_next` so no `futures_util`
/// (dev-only) leaks into the library build.
async fn drain_input(
    mut stream: Pin<Box<dyn Stream<Item = String> + Send + 'static>>,
    out_tx: mpsc::UnboundedSender<String>,
) {
    while let Some(text) = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
        let Ok(line) = encode_line(&user_message_frame(&text)) else {
            break;
        };
        if out_tx.send(line).is_err() {
            break;
        }
    }
}

/// Probe `program --version`, returning the trimmed stdout if it ran.
async fn probe_version(program: &std::path::Path) -> Option<String> {
    let output = tokio::process::Command::new(program)
        .arg("--version")
        .output()
        .await
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Send the initialize request and read frames until its control response.
///
/// Bounded by the same [`Options::control_request_timeout`] that guards every
/// later control request: the handshake is itself a control request/response
/// round trip (`initialize`), so a binary that never answers it must not
/// hang `connect()` forever either.
async fn handshake(
    stdin: &mut ChildStdin,
    stdout: &mut StdoutLines,
    options: &Options,
    id_gen: &RequestIdGen,
) -> Result<Capabilities, AgentError> {
    let id = id_gen.next();
    let request = initialize_request(options, id.as_str());
    let line = encode_line(&request)?;
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|_| AgentError::TransportClosed)?;
    stdin
        .flush()
        .await
        .map_err(|_| AgentError::TransportClosed)?;

    let timeout = options.control_request_timeout;
    match tokio::time::timeout(timeout, read_initialize_response(stdout)).await {
        Ok(result) => result,
        Err(_elapsed) => Err(AgentError::ControlRequestTimedOut {
            method: "initialize".to_string(),
            elapsed: timeout,
        }),
    }
}

/// Read frames from `stdout` until the initialize response arrives.
async fn read_initialize_response(stdout: &mut StdoutLines) -> Result<Capabilities, AgentError> {
    loop {
        match stdout.next_line().await {
            Ok(Some(text)) if text.trim().is_empty() => {}
            Ok(Some(text)) => {
                if let crate::agent::protocol::InboundFrame::ControlResponse(response) =
                    decode_inbound(&text)?
                {
                    return control_response_outcome(response.response, "initialize")
                        .map(|value| parse_capabilities(&value));
                }
                // Ignore any pre-handshake message frames.
            }
            Ok(None) | Err(_) => return Err(AgentError::TransportClosed),
        }
    }
}

/// The single outbound writer: owns stdin, drains pre-encoded lines.
async fn writer_loop(mut stdin: ChildStdin, mut rx: mpsc::UnboundedReceiver<String>) {
    while let Some(line) = rx.recv().await {
        if stdin.write_all(line.as_bytes()).await.is_err() {
            break;
        }
        if stdin.flush().await.is_err() {
            break;
        }
    }
}

/// The background reader: decode each line, dispatch control requests, and
/// demultiplex everything else.
async fn reader_loop(mut stdout: StdoutLines, demux: Arc<Demux>, dispatcher: Arc<Dispatcher>) {
    loop {
        #[expect(
            clippy::match_same_arms,
            reason = "Ok(None) and Err(LineError::Io(_)) share a body today but are kept as \
                      separate arms so each terminal condition (clean EOF vs. I/O failure) reads \
                      distinctly from the Overflow error path"
        )]
        match stdout.next_line().await {
            Ok(Some(text)) if text.trim().is_empty() => {}
            Ok(Some(text)) => match decode_inbound(&text) {
                Ok(crate::agent::protocol::InboundFrame::ControlRequest(req)) => {
                    // Spawn so a slow handler never stalls the reader.
                    let dispatcher = Arc::clone(&dispatcher);
                    tokio::spawn(async move { dispatcher.handle(req).await });
                }
                Ok(frame) => demux.route(frame).await,
                Err(error) => demux.fail_turn(error).await,
            },
            Ok(None) => {
                demux.close().await;
                break;
            }
            Err(LineError::Overflow { cap }) => {
                demux
                    .fail_turn(AgentError::Protocol {
                        detail: format!("stdout line exceeded max_buffer_size ({cap} bytes)"),
                    })
                    .await;
                demux.close().await;
                break;
            }
            Err(LineError::Io(_)) => {
                demux.close().await;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "test asserts known-valid channel receives"
    )]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use std::time::Duration;

    use super::{
        await_control_response, control_response_outcome, drain_input, user_message_frame,
    };
    use crate::agent::error::AgentError;
    use crate::agent::protocol::ControlResponseBody;

    #[test]
    fn control_response_outcome_maps_success_to_ok() {
        let response = ControlResponseBody::Success {
            request_id: "req_1".to_string(),
            response: serde_json::json!({"ok": true}),
        };
        let result = control_response_outcome(response, "interrupt").expect("ok");
        assert_eq!(result, serde_json::json!({"ok": true}));
    }

    #[test]
    fn control_response_outcome_maps_error_to_control_request_failed() {
        let response = ControlResponseBody::Error {
            request_id: "req_1".to_string(),
            error: "denied".to_string(),
        };
        let err = control_response_outcome(response, "interrupt").expect_err("err");
        match err {
            AgentError::ControlRequestFailed { method, detail } => {
                assert_eq!(method, "interrupt");
                assert_eq!(detail, "denied");
            }
            other => panic!("expected ControlRequestFailed, got {other:?}"),
        }
    }

    #[test]
    fn control_response_outcome_maps_malformed_to_error_instead_of_hanging() {
        // The outbound mirror of the inbound dispatcher's `Malformed` arm: a
        // response body that failed to deserialize still resolves the
        // waiter, with an error, rather than leaving `send_control`'s caller
        // blocked forever.
        let response = ControlResponseBody::Malformed {
            request_id: "req_1".to_string(),
            detail: "unmodeled subtype".to_string(),
        };
        let err = control_response_outcome(response, "interrupt").expect_err("err");
        match err {
            AgentError::ControlRequestFailed { method, detail } => {
                assert_eq!(method, "interrupt");
                assert_eq!(detail, "unmodeled subtype");
            }
            other => panic!("expected ControlRequestFailed, got {other:?}"),
        }
    }

    #[test]
    fn user_message_frame_wraps_prompt_text() {
        let value = user_message_frame("hello there");
        assert_eq!(value["type"], "user");
        assert_eq!(value["message"]["role"], "user");
        assert_eq!(value["message"]["content"], "hello there");
    }

    #[test]
    fn user_message_frame_is_unchanged_by_writer_refactor() {
        let value = user_message_frame("hi");
        assert_eq!(value["type"], "user");
        assert_eq!(value["message"]["content"], "hi");
    }

    #[tokio::test(start_paused = true)]
    async fn await_control_response_times_out_without_a_real_wait() {
        // A response that never arrives must not hang the caller past the
        // configured bound. `tokio::time::timeout` only arms its deadline on
        // first poll, so poll the future once here (confirming it parks)
        // before advancing virtual time past the bound. That poll-then-
        // advance sequence is sufficient on its own to fire the timeout
        // below, though it is not the only thing that would: tokio's idle
        // auto-advance under `start_paused` fires the same deadline
        // unprompted once every task is blocked, regardless of whether this
        // test drives the advance itself.
        use std::future::Future;

        let (_tx, rx) = tokio::sync::oneshot::channel::<ControlResponseBody>();
        let mut fut = std::pin::pin!(await_control_response(
            rx,
            Duration::from_secs(60),
            "interrupt"
        ));

        let first_poll =
            std::future::poll_fn(|cx| std::task::Poll::Ready(fut.as_mut().poll(cx))).await;
        assert!(
            first_poll.is_pending(),
            "must not resolve before the deadline is armed"
        );

        tokio::time::advance(Duration::from_secs(61)).await;

        // Bounded the same way demux.rs guards its malformed-response and
        // close() tests: if the timeout logic under test regressed to a
        // bare, timer-less `rx.await`, awaiting `fut` directly would hang
        // indefinitely instead of failing. This outer timeout has its own
        // deadline for tokio's idle auto-advance under `start_paused` to
        // fire even when `fut` itself has no timer left to drive it.
        let err = tokio::time::timeout(Duration::from_secs(1), fut)
            .await
            .expect("guard: must not hang if the control-response timeout regresses")
            .expect_err("must time out, not hang");
        match err {
            AgentError::ControlRequestTimedOut { method, elapsed } => {
                assert_eq!(method, "interrupt");
                assert_eq!(elapsed, Duration::from_secs(60));
            }
            other => panic!("expected ControlRequestTimedOut, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn await_control_response_maps_dropped_sender_to_transport_closed() {
        let (tx, rx) = tokio::sync::oneshot::channel::<ControlResponseBody>();
        drop(tx);
        let err = await_control_response(rx, Duration::from_secs(60), "interrupt")
            .await
            .expect_err("dropped sender must not resolve to a value");
        assert!(matches!(err, AgentError::TransportClosed));
    }

    #[tokio::test]
    async fn await_control_response_passes_through_a_timely_response() {
        let (tx, rx) = tokio::sync::oneshot::channel::<ControlResponseBody>();
        tx.send(ControlResponseBody::Success {
            request_id: "req_1".to_string(),
            response: serde_json::json!({"ok": true}),
        })
        .expect("send");
        let value = await_control_response(rx, Duration::from_secs(60), "interrupt")
            .await
            .expect("ok");
        assert_eq!(value, serde_json::json!({"ok": true}));
    }

    #[tokio::test]
    async fn drain_input_feeds_each_item_as_user_message_line() {
        use futures_util::StreamExt;

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let stream = futures_util::stream::iter(vec!["a".to_string(), "b".to_string()]).boxed();
        drain_input(stream, tx).await;

        let first = rx.recv().await.expect("first line");
        let second = rx.recv().await.expect("second line");
        assert!(first.contains("\"content\":\"a\""));
        assert!(second.contains("\"content\":\"b\""));
        assert!(rx.recv().await.is_none());
    }
}
