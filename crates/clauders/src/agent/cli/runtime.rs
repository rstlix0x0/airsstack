//! The subprocess-backed `Runtime` implementation.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::agent::capabilities::Capabilities;
use crate::agent::error::AgentError;
use crate::agent::options::Options;
use crate::agent::permissions::PermissionMode;
use crate::agent::process::{ManagedProcess, ProcessConfig, ProcessIo, StdoutLines};
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
        let dispatcher = Arc::new(Dispatcher::new(hooks, policy, out_tx.clone()));

        let demux = Arc::new(Demux::new());
        let reader = tokio::spawn(reader_loop(stdout, Arc::clone(&demux), dispatcher));
        // stderr is drained by the process layer; not needed for message routing.
        drop(stderr);

        Ok(Self {
            out_tx,
            demux,
            id_gen,
            capabilities,
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

        let (tx, rx) = oneshot::channel();
        self.demux.register_pending(id.as_str().to_string(), tx);

        if self.out_tx.send(line).is_err() {
            self.demux.remove_pending(id.as_str());
            return Err(AgentError::TransportClosed);
        }

        match rx.await {
            Ok(ControlResponseBody::Success { response, .. }) => Ok(response),
            Ok(ControlResponseBody::Error { error, .. }) => Err(AgentError::ControlRequestFailed {
                method: method.to_string(),
                detail: error,
            }),
            Err(_) => Err(AgentError::TransportClosed),
        }
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
        let line = encode_line(&user_message_frame(&prompt))?;
        if self.out_tx.send(line).is_err() {
            return Err(AgentError::TransportClosed);
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

/// Build the outbound user-message frame carrying a prompt.
fn user_message_frame(prompt: &Prompt) -> serde_json::Value {
    serde_json::json!({
        "type": "user",
        "message": { "role": "user", "content": prompt.as_str() }
    })
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

    loop {
        match stdout.next_line().await {
            Ok(Some(text)) if text.trim().is_empty() => {}
            Ok(Some(text)) => {
                if let crate::agent::protocol::InboundFrame::ControlResponse(response) =
                    decode_inbound(&text)?
                {
                    return match response.response {
                        ControlResponseBody::Success { response, .. } => {
                            Ok(parse_capabilities(&response))
                        }
                        ControlResponseBody::Error { error, .. } => {
                            Err(AgentError::ControlRequestFailed {
                                method: "initialize".to_string(),
                                detail: error,
                            })
                        }
                    };
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
            Ok(None) | Err(_) => {
                demux.close().await;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::user_message_frame;
    use crate::agent::types::Prompt;

    #[test]
    fn user_message_frame_wraps_prompt_text() {
        let value = user_message_frame(&Prompt::new("hello there"));
        assert_eq!(value["type"], "user");
        assert_eq!(value["message"]["role"], "user");
        assert_eq!(value["message"]["content"], "hello there");
    }

    #[test]
    fn user_message_frame_is_unchanged_by_writer_refactor() {
        let value = user_message_frame(&Prompt::new("hi"));
        assert_eq!(value["type"], "user");
        assert_eq!(value["message"]["content"], "hi");
    }
}
