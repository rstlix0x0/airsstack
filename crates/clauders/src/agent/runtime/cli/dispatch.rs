//! Dispatch of inbound control requests to registered handlers.
//!
//! The reader task intercepts each inbound `control_request` and hands it to a
//! [`Dispatcher`], which consults the registered [`PermissionPolicy`],
//! [`Hook`], MCP server, or elicitation policy, encodes the correlated control
//! response, and enqueues it on the outbound line channel drained by the
//! writer task. A handler error becomes an error control response so the
//! binary is never left waiting.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::mpsc;

use crate::agent::cancel::CancelSignal;
use crate::agent::elicitation::{ElicitationPolicy, ElicitationRequest, ElicitationResponse};
use crate::agent::hooks::{HookInput, HookRegistry};
use crate::agent::mcp::{SdkMcpRegistry, router};
use crate::agent::permissions::{PermissionContext, PermissionDecision, PermissionPolicy};
use crate::agent::protocol::{
    InboundControlRequest, InboundRequestBody, OutboundControlResponse, OutboundResponseBody,
    encode_line,
};

/// Handles inbound control requests by invoking registered handlers.
pub(super) struct Dispatcher {
    hooks: Arc<HookRegistry>,
    policy: Option<Arc<dyn PermissionPolicy>>,
    elicitation: Option<Arc<dyn ElicitationPolicy>>,
    mcp: Arc<SdkMcpRegistry>,
    out_tx: mpsc::UnboundedSender<String>,
    /// Cancellation signals for requests currently being serviced, keyed by
    /// the binary's `request_id`.
    ///
    /// The guard is never held across an `.await`: every access is a single
    /// lock, act, drop.
    in_flight: Mutex<HashMap<String, CancelSignal>>,
}

impl Dispatcher {
    /// Build a dispatcher over the session's handlers and outbound channel.
    pub(super) fn new(
        hooks: Arc<HookRegistry>,
        policy: Option<Arc<dyn PermissionPolicy>>,
        elicitation: Option<Arc<dyn ElicitationPolicy>>,
        mcp: Arc<SdkMcpRegistry>,
        out_tx: mpsc::UnboundedSender<String>,
    ) -> Self {
        Self {
            hooks,
            policy,
            elicitation,
            mcp,
            out_tx,
            in_flight: Mutex::new(HashMap::new()),
        }
    }

    /// Claim `request_id`, returning the handler's cancellation signal.
    ///
    /// Returns `None` when the id is already being serviced. The binary
    /// replays in-flight requests to a client that joins an initialized
    /// session, so the same id can arrive twice; running the handler twice
    /// would prompt the user twice and answer once per run.
    ///
    /// The check and the insert are one locked operation on purpose —
    /// splitting them reopens the race this closes.
    pub(super) fn begin(&self, request_id: &str) -> Option<CancelSignal> {
        let mut guard = self
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if guard.contains_key(request_id) {
            drop(guard);
            tracing::warn!(
                request_id,
                "duplicate delivery of an in-flight control request; skipping"
            );
            return None;
        }
        let signal = CancelSignal::new();
        guard.insert(request_id.to_string(), signal.clone());
        drop(guard);
        Some(signal)
    }

    /// Release `request_id` once its handler has finished.
    fn finish(&self, request_id: &str) {
        self.in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(request_id);
    }

    /// Cancel the handler servicing `request_id`, if one is still running.
    ///
    /// A cancel for an unknown or already-finished id is an expected race,
    /// not a fault.
    pub(super) fn cancel(&self, request_id: &str) {
        let signal = self
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(request_id);
        if let Some(signal) = signal {
            signal.cancel();
        } else {
            tracing::debug!(request_id, "cancel for a request that is no longer running");
        }
    }

    /// Handle one inbound control request end to end.
    pub(super) async fn handle(&self, req: InboundControlRequest, cancel: CancelSignal) {
        let request_id = req.request_id;
        let outcome = match req.request {
            InboundRequestBody::CanUseTool {
                tool_name,
                input,
                tool_use_id,
                agent_id,
                blocked_path,
                decision_reason,
                title,
                display_name,
                description,
                permission_suggestions,
                matched_ask_rule,
            } => {
                let ctx = PermissionContext {
                    tool_use_id,
                    agent_id,
                    blocked_path,
                    decision_reason,
                    title,
                    display_name,
                    description,
                    suggestions: permission_suggestions,
                    matched_ask_rule,
                    request_id: request_id.clone(),
                };
                self.permission_outcome(&tool_name, input, ctx, cancel)
                    .await
            }
            InboundRequestBody::HookCallback {
                callback_id,
                input,
                tool_use_id,
            } => {
                self.hook_outcome(&callback_id, input, tool_use_id, cancel)
                    .await
            }
            InboundRequestBody::McpMessage {
                server_name,
                message,
            } => self.mcp_outcome(&server_name, message).await,
            InboundRequestBody::Elicitation {
                elicitation_id,
                message,
                mode,
                requested_schema,
                url,
                mcp_server_name,
                title,
                display_name,
                description,
            } => {
                let request = ElicitationRequest {
                    elicitation_id,
                    message,
                    mode,
                    requested_schema,
                    url,
                    mcp_server_name,
                    title,
                    display_name,
                    description,
                };
                self.elicitation_outcome(request, cancel).await
            }
            // A recognized-but-malformed control request body: report which
            // subtype failed to deserialize rather than failing the whole
            // turn.
            InboundRequestBody::Malformed { subtype } => Err(format!(
                "malformed control request (subtype: {})",
                subtype.as_deref().unwrap_or("absent")
            )),
            // An unmodeled control-request subtype degrades to an error
            // response rather than failing the whole turn.
            InboundRequestBody::Other => Err("unsupported control request subtype".to_string()),
        };
        self.write_response(request_id.clone(), outcome);
        self.finish(&request_id);
    }

    /// Resolve a `can_use_tool` request to its response payload.
    async fn permission_outcome(
        &self,
        tool: &str,
        input: serde_json::Value,
        ctx: PermissionContext,
        cancel: CancelSignal,
    ) -> Result<serde_json::Value, String> {
        match &self.policy {
            Some(policy) => match policy.can_use_tool(tool, &input, ctx, cancel).await {
                Ok(decision) => decision
                    .into_response_value(&input)
                    .map_err(|e| e.to_string()),
                Err(err) => Err(err.to_string()),
            },
            // No policy registered: allow, echoing the original input.
            None => PermissionDecision::allow()
                .into_response_value(&input)
                .map_err(|e| e.to_string()),
        }
    }

    /// Resolve an `elicitation` request to its response payload.
    async fn elicitation_outcome(
        &self,
        request: ElicitationRequest,
        cancel: CancelSignal,
    ) -> Result<serde_json::Value, String> {
        match &self.elicitation {
            Some(policy) => match policy.elicit(request, cancel).await {
                Ok(response) => Ok(response.into_response_value()),
                Err(err) => Err(err.to_string()),
            },
            // No policy registered: decline, matching the official SDK.
            None => Ok(ElicitationResponse::Decline.into_response_value()),
        }
    }

    /// Resolve a `hook_callback` request to its response payload.
    async fn hook_outcome(
        &self,
        callback_id: &str,
        data: serde_json::Value,
        tool_use_id: Option<String>,
        cancel: CancelSignal,
    ) -> Result<serde_json::Value, String> {
        match self.hooks.lookup(callback_id) {
            Some((event, hook)) => {
                let input = HookInput {
                    event,
                    tool_use_id,
                    data,
                };
                match hook.call(input, cancel).await {
                    Ok(output) => {
                        serde_json::to_value(output).map_err(|e: serde_json::Error| e.to_string())
                    }
                    Err(err) => Err(err.to_string()),
                }
            }
            // Unknown callback id: no-op empty response rather than hang.
            None => Ok(serde_json::json!({})),
        }
    }

    /// Resolve an `mcp_message` request against the named in-process server.
    async fn mcp_outcome(
        &self,
        server_name: &str,
        message: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match self.mcp.lookup(server_name) {
            Some(server) => {
                let mcp_response = router::handle_mcp_message(&server, &message).await;
                Ok(serde_json::json!({ "mcp_response": mcp_response }))
            }
            None => Err(format!("unknown mcp server: {server_name}")),
        }
    }

    /// Encode and enqueue the control response for `request_id`.
    fn write_response(&self, request_id: String, outcome: Result<serde_json::Value, String>) {
        let body = match outcome {
            Ok(response) => OutboundResponseBody::Success {
                request_id,
                response,
            },
            Err(error) => OutboundResponseBody::Error { request_id, error },
        };
        let frame = OutboundControlResponse {
            kind: "control_response",
            response: body,
        };
        match encode_line(&frame) {
            Ok(line) => {
                // Receiver dropped only on shutdown; ignore the send error.
                let _ = self.out_tx.send(line);
            }
            Err(err) => {
                tracing::error!(error = %err, "failed to encode control response");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::sync::Arc;

    use tokio::sync::mpsc;

    use super::Dispatcher;
    use crate::agent::cancel::CancelSignal;
    use crate::agent::elicitation::{ElicitationPolicy, ElicitationRequest, ElicitationResponse};
    use crate::agent::error::AgentError;
    use crate::agent::hooks::HookRegistry;
    use crate::agent::mcp::SdkMcpRegistry;
    use crate::agent::permissions::{PermissionContext, PermissionDecision, PermissionPolicy};
    use crate::agent::protocol::decode_inbound;

    struct AllowPolicy;

    #[async_trait::async_trait]
    impl PermissionPolicy for AllowPolicy {
        async fn can_use_tool(
            &self,
            _tool: &str,
            _input: &serde_json::Value,
            _ctx: PermissionContext,
            _cancel: CancelSignal,
        ) -> Result<PermissionDecision, AgentError> {
            Ok(PermissionDecision::allow())
        }
    }

    struct DenyPolicy;

    #[async_trait::async_trait]
    impl PermissionPolicy for DenyPolicy {
        async fn can_use_tool(
            &self,
            _tool: &str,
            _input: &serde_json::Value,
            _ctx: PermissionContext,
            _cancel: CancelSignal,
        ) -> Result<PermissionDecision, AgentError> {
            Ok(PermissionDecision::deny("nope"))
        }
    }

    fn can_use_tool_request() -> crate::agent::protocol::InboundControlRequest {
        let frame = decode_inbound(
            r#"{"type":"control_request","request_id":"srv_1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"cmd":"ls"}}}"#,
        )
        .expect("decode");
        match frame {
            crate::agent::protocol::InboundFrame::ControlRequest(req) => req,
            _ => unreachable!("decoded a control request"),
        }
    }

    #[tokio::test]
    async fn allow_policy_writes_allow_response_echoing_input() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(HookRegistry::default()),
            Some(Arc::new(AllowPolicy)),
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        dispatcher
            .handle(can_use_tool_request(), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["type"], "control_response");
        assert_eq!(value["response"]["subtype"], "success");
        assert_eq!(value["response"]["request_id"], "srv_1");
        assert_eq!(value["response"]["response"]["behavior"], "allow");
        assert_eq!(value["response"]["response"]["updatedInput"]["cmd"], "ls");
    }

    #[tokio::test]
    async fn deny_policy_writes_deny_response() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(HookRegistry::default()),
            Some(Arc::new(DenyPolicy)),
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        dispatcher
            .handle(can_use_tool_request(), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["response"]["behavior"], "deny");
        assert_eq!(value["response"]["response"]["message"], "nope");
        assert_eq!(value["response"]["response"]["interrupt"], false);
    }

    struct CancelObservingPolicy;

    #[async_trait::async_trait]
    impl PermissionPolicy for CancelObservingPolicy {
        async fn can_use_tool(
            &self,
            _tool: &str,
            _input: &serde_json::Value,
            _ctx: PermissionContext,
            cancel: CancelSignal,
        ) -> Result<PermissionDecision, AgentError> {
            cancel.cancelled().await;
            Err(AgentError::Protocol {
                detail: "cancelled".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn cancel_reaches_the_handler_that_is_running() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Arc::new(Dispatcher::new(
            Arc::new(HookRegistry::default()),
            Some(Arc::new(CancelObservingPolicy)),
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        ));

        let req = can_use_tool_request();
        let id = req.request_id.clone();
        let signal = dispatcher.begin(&id).expect("first delivery registers");
        let running = {
            let dispatcher = Arc::clone(&dispatcher);
            tokio::spawn(async move { dispatcher.handle(req, signal).await })
        };

        dispatcher.cancel(&id);
        running.await.expect("handler task");

        let line = rx.recv().await.expect("a response line");
        assert!(
            line.contains("\"subtype\":\"error\""),
            "an observing handler that bails must produce an error response: {line}"
        );
    }

    #[test]
    fn a_duplicate_delivery_is_refused() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(HookRegistry::default()),
            None,
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        assert!(
            dispatcher.begin("r1").is_some(),
            "first delivery is accepted"
        );
        assert!(
            dispatcher.begin("r1").is_none(),
            "a replayed request_id must not run the handler twice"
        );
    }

    #[tokio::test]
    async fn a_finished_request_frees_its_slot() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Arc::new(Dispatcher::new(
            Arc::new(HookRegistry::default()),
            Some(Arc::new(AllowPolicy)),
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        ));
        let req = can_use_tool_request();
        let id = req.request_id.clone();
        let signal = dispatcher.begin(&id).expect("registers");
        dispatcher.handle(req, signal).await;
        let _ = rx.recv().await;
        assert!(
            dispatcher.begin(&id).is_some(),
            "the slot must be released when the handler finishes"
        );
    }

    #[test]
    fn cancelling_an_unknown_request_is_a_no_op() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(HookRegistry::default()),
            None,
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        dispatcher.cancel("never-seen");
    }

    #[tokio::test]
    async fn no_policy_allows_with_original_input() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(HookRegistry::default()),
            None,
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        dispatcher
            .handle(can_use_tool_request(), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["response"]["behavior"], "allow");
        assert_eq!(value["response"]["response"]["updatedInput"]["cmd"], "ls");
    }

    use crate::agent::capabilities::HookEvent;
    use crate::agent::hooks::{Hook, HookInput, HookOutput};

    struct BlockingHook;

    #[async_trait::async_trait]
    impl Hook for BlockingHook {
        async fn call(
            &self,
            _input: HookInput,
            _cancel: CancelSignal,
        ) -> Result<HookOutput, AgentError> {
            Ok(HookOutput {
                decision: Some(crate::agent::hooks::HookDecision::Block),
                reason: Some("denied".to_string()),
                ..HookOutput::default()
            })
        }
    }

    struct FailingHook;

    #[async_trait::async_trait]
    impl Hook for FailingHook {
        async fn call(
            &self,
            _input: HookInput,
            _cancel: CancelSignal,
        ) -> Result<HookOutput, AgentError> {
            Err(AgentError::Protocol {
                detail: "boom".to_string(),
            })
        }
    }

    fn hook_request(callback_id: &str) -> crate::agent::protocol::InboundControlRequest {
        let line = format!(
            r#"{{"type":"control_request","request_id":"srv_2","request":{{"subtype":"hook_callback","callback_id":"{callback_id}","input":{{"k":"v"}}}}}}"#
        );
        match decode_inbound(&line).expect("decode") {
            crate::agent::protocol::InboundFrame::ControlRequest(req) => req,
            _ => unreachable!("decoded a control request"),
        }
    }

    #[tokio::test]
    async fn registered_hook_response_is_serialized() {
        let mut reg = HookRegistry::default();
        reg.register(HookEvent::PreToolUse, None, Arc::new(BlockingHook));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(reg),
            None,
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        dispatcher
            .handle(hook_request("hook_0"), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "success");
        assert_eq!(value["response"]["response"]["decision"], "block");
        assert_eq!(value["response"]["response"]["reason"], "denied");
    }

    #[tokio::test]
    async fn unknown_callback_id_is_empty_success() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(HookRegistry::default()),
            None,
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        dispatcher
            .handle(hook_request("hook_404"), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "success");
        assert_eq!(value["response"]["response"], serde_json::json!({}));
    }

    fn mcp_message_request(
        server: &str,
        tool: &str,
    ) -> crate::agent::protocol::InboundControlRequest {
        let line = format!(
            r#"{{"type":"control_request","request_id":"srv_9","request":{{"subtype":"mcp_message","server_name":"{server}","message":{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{{}}}}}}}}}}"#
        );
        match decode_inbound(&line).expect("decode") {
            crate::agent::protocol::InboundFrame::ControlRequest(req) => req,
            _ => unreachable!("decoded a control request"),
        }
    }

    #[tokio::test]
    async fn mcp_message_routes_to_registered_server() {
        use crate::agent::mcp::{SdkMcpServer, ToolResult, tool};
        let echo = tool(
            "echo",
            "echo",
            serde_json::json!({"type":"object"}),
            |_| async { Ok(ToolResult::text("hi")) },
        );
        let mut reg = SdkMcpRegistry::default();
        reg.register(SdkMcpServer::builder("calc").tool(echo).build());
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(HookRegistry::default()),
            None,
            None,
            Arc::new(reg),
            tx,
        );
        dispatcher
            .handle(mcp_message_request("calc", "echo"), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "success");
        assert_eq!(value["response"]["request_id"], "srv_9");
        assert_eq!(
            value["response"]["response"]["mcp_response"]["result"]["content"][0]["text"],
            "hi"
        );
    }

    #[tokio::test]
    async fn mcp_message_unknown_server_is_error() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(HookRegistry::default()),
            None,
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        dispatcher
            .handle(mcp_message_request("ghost", "echo"), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "error");
        assert!(
            value["response"]["error"]
                .as_str()
                .expect("error")
                .contains("unknown mcp server")
        );
    }

    #[tokio::test]
    async fn hook_error_becomes_error_response() {
        let mut reg = HookRegistry::default();
        reg.register(HookEvent::Stop, None, Arc::new(FailingHook));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(
            Arc::new(reg),
            None,
            None,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        );
        dispatcher
            .handle(hook_request("hook_0"), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "error");
        assert!(
            value["response"]["error"]
                .as_str()
                .expect("error string")
                .contains("boom"),
            "error detail should carry the handler message"
        );
    }

    struct AcceptElicitation;

    #[async_trait::async_trait]
    impl ElicitationPolicy for AcceptElicitation {
        async fn elicit(
            &self,
            _r: ElicitationRequest,
            _cancel: CancelSignal,
        ) -> Result<ElicitationResponse, AgentError> {
            Ok(ElicitationResponse::Accept(
                serde_json::json!({ "branch": "main" }),
            ))
        }
    }

    struct FailingElicitation;

    #[async_trait::async_trait]
    impl ElicitationPolicy for FailingElicitation {
        async fn elicit(
            &self,
            _r: ElicitationRequest,
            _cancel: CancelSignal,
        ) -> Result<ElicitationResponse, AgentError> {
            Err(AgentError::Protocol {
                detail: "elicit boom".to_string(),
            })
        }
    }

    fn elicitation_request() -> crate::agent::protocol::InboundControlRequest {
        let line = r#"{"type":"control_request","request_id":"srv_20","request":{"subtype":"elicitation","elicitation_id":"elic_1","message":"Pick","mode":"form","requested_schema":{"type":"object"},"mcp_server_name":"git"}}"#;
        match decode_inbound(line).expect("decode") {
            crate::agent::protocol::InboundFrame::ControlRequest(req) => req,
            _ => unreachable!("decoded a control request"),
        }
    }

    fn dispatcher_with_elicitation(
        policy: Option<Arc<dyn ElicitationPolicy>>,
        tx: mpsc::UnboundedSender<String>,
    ) -> Dispatcher {
        Dispatcher::new(
            Arc::new(HookRegistry::default()),
            None,
            policy,
            Arc::new(SdkMcpRegistry::default()),
            tx,
        )
    }

    #[tokio::test]
    async fn accept_handler_writes_action_and_content() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = dispatcher_with_elicitation(Some(Arc::new(AcceptElicitation)), tx);
        dispatcher
            .handle(elicitation_request(), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "success");
        assert_eq!(value["response"]["request_id"], "srv_20");
        assert_eq!(value["response"]["response"]["action"], "accept");
        assert_eq!(value["response"]["response"]["content"]["branch"], "main");
    }

    #[tokio::test]
    async fn no_elicitation_policy_declines() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = dispatcher_with_elicitation(None, tx);
        dispatcher
            .handle(elicitation_request(), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "success");
        assert_eq!(value["response"]["response"]["action"], "decline");
    }

    #[tokio::test]
    async fn elicitation_policy_error_becomes_error_response() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = dispatcher_with_elicitation(Some(Arc::new(FailingElicitation)), tx);
        dispatcher
            .handle(elicitation_request(), CancelSignal::new())
            .await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "error");
        assert!(
            value["response"]["error"]
                .as_str()
                .expect("error string")
                .contains("elicit boom")
        );
    }

    #[tokio::test]
    async fn malformed_control_request_becomes_error_response_naming_subtype() {
        // Missing the required `mcp_server_name` field: recognized subtype,
        // malformed body.
        let line = r#"{"type":"control_request","request_id":"srv_22","request":{"subtype":"elicitation","message":"Pick","mode":"form"}}"#;
        let crate::agent::protocol::InboundFrame::ControlRequest(req) =
            decode_inbound(line).expect("decode")
        else {
            unreachable!("decoded a control request")
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = dispatcher_with_elicitation(None, tx);
        dispatcher.handle(req, CancelSignal::new()).await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "error");
        assert_eq!(value["response"]["request_id"], "srv_22");
        assert!(
            value["response"]["error"]
                .as_str()
                .expect("error string")
                .contains("elicitation"),
            "error detail should name the malformed subtype"
        );
    }

    #[tokio::test]
    async fn malformed_control_request_with_no_subtype_names_it_absent() {
        // A `request` body with no `subtype` at all: the `Malformed { subtype:
        // None }` half of the arm, not exercised by the `Some(subtype)` case
        // covered above.
        let line = r#"{"type":"control_request","request_id":"srv_23","request":{}}"#;
        let crate::agent::protocol::InboundFrame::ControlRequest(req) =
            decode_inbound(line).expect("decode")
        else {
            unreachable!("decoded a control request")
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = dispatcher_with_elicitation(None, tx);
        dispatcher.handle(req, CancelSignal::new()).await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "error");
        assert_eq!(value["response"]["request_id"], "srv_23");
        assert!(
            value["response"]["error"]
                .as_str()
                .expect("error string")
                .contains("absent"),
            "error detail should report the missing subtype as absent"
        );
    }

    #[tokio::test]
    async fn unsupported_subtype_becomes_error_response() {
        let line = r#"{"type":"control_request","request_id":"srv_21","request":{"subtype":"some_future_subtype"}}"#;
        let crate::agent::protocol::InboundFrame::ControlRequest(req) =
            decode_inbound(line).expect("decode")
        else {
            unreachable!("decoded a control request")
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = dispatcher_with_elicitation(None, tx);
        dispatcher.handle(req, CancelSignal::new()).await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "error");
        assert!(
            value["response"]["error"]
                .as_str()
                .expect("error string")
                .contains("unsupported control request")
        );
    }
}
