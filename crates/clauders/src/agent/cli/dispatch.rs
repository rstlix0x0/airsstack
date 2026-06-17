//! Dispatch of inbound control requests to registered handlers.
//!
//! The reader task intercepts each inbound `control_request` and hands it to a
//! [`Dispatcher`], which consults the registered [`PermissionPolicy`] or
//! [`Hook`], encodes the correlated control response, and enqueues it on the
//! outbound line channel drained by the writer task. A handler error becomes
//! an error control response so the binary is never left waiting.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::agent::hooks::{HookInput, HookRegistry};
use crate::agent::permissions::{PermissionContext, PermissionDecision, PermissionPolicy};
use crate::agent::protocol::{
    InboundControlRequest, InboundRequestBody, OutboundControlResponse, OutboundResponseBody,
    encode_line,
};

/// Handles inbound control requests by invoking registered handlers.
pub(super) struct Dispatcher {
    hooks: Arc<HookRegistry>,
    policy: Option<Arc<dyn PermissionPolicy>>,
    out_tx: mpsc::UnboundedSender<String>,
}

impl Dispatcher {
    /// Build a dispatcher over the session's handlers and outbound channel.
    pub(super) fn new(
        hooks: Arc<HookRegistry>,
        policy: Option<Arc<dyn PermissionPolicy>>,
        out_tx: mpsc::UnboundedSender<String>,
    ) -> Self {
        Self {
            hooks,
            policy,
            out_tx,
        }
    }

    /// Handle one inbound control request end to end.
    pub(super) async fn handle(&self, req: InboundControlRequest) {
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
            } => {
                let ctx = PermissionContext {
                    tool_use_id,
                    agent_id,
                    blocked_path,
                    decision_reason,
                    title,
                    display_name,
                    description,
                };
                self.permission_outcome(&tool_name, input, ctx).await
            }
            InboundRequestBody::HookCallback {
                callback_id,
                input,
                tool_use_id,
            } => self.hook_outcome(&callback_id, input, tool_use_id).await,
        };
        self.write_response(request_id, outcome);
    }

    /// Resolve a `can_use_tool` request to its response payload.
    async fn permission_outcome(
        &self,
        tool: &str,
        input: serde_json::Value,
        ctx: PermissionContext,
    ) -> Result<serde_json::Value, String> {
        match &self.policy {
            Some(policy) => match policy.can_use_tool(tool, &input, ctx).await {
                Ok(decision) => Ok(decision.into_response_value(&input)),
                Err(err) => Err(err.to_string()),
            },
            // No policy registered: allow, echoing the original input.
            None => Ok(PermissionDecision::Allow {
                updated_input: None,
            }
            .into_response_value(&input)),
        }
    }

    /// Resolve a `hook_callback` request to its response payload.
    async fn hook_outcome(
        &self,
        callback_id: &str,
        data: serde_json::Value,
        tool_use_id: Option<String>,
    ) -> Result<serde_json::Value, String> {
        match self.hooks.lookup(callback_id) {
            Some((event, hook)) => {
                let input = HookInput {
                    event,
                    tool_use_id,
                    data,
                };
                match hook.call(input).await {
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
    use crate::agent::error::AgentError;
    use crate::agent::hooks::HookRegistry;
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
        ) -> Result<PermissionDecision, AgentError> {
            Ok(PermissionDecision::Allow {
                updated_input: None,
            })
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
        ) -> Result<PermissionDecision, AgentError> {
            Ok(PermissionDecision::Deny {
                message: "nope".to_string(),
            })
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
            tx,
        );
        dispatcher.handle(can_use_tool_request()).await;
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
            tx,
        );
        dispatcher.handle(can_use_tool_request()).await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["response"]["behavior"], "deny");
        assert_eq!(value["response"]["response"]["message"], "nope");
    }

    #[tokio::test]
    async fn no_policy_allows_with_original_input() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(Arc::new(HookRegistry::default()), None, tx);
        dispatcher.handle(can_use_tool_request()).await;
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
        async fn call(&self, _input: HookInput) -> Result<HookOutput, AgentError> {
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
        async fn call(&self, _input: HookInput) -> Result<HookOutput, AgentError> {
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
        let dispatcher = Dispatcher::new(Arc::new(reg), None, tx);
        dispatcher.handle(hook_request("hook_0")).await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "success");
        assert_eq!(value["response"]["response"]["decision"], "block");
        assert_eq!(value["response"]["response"]["reason"], "denied");
    }

    #[tokio::test]
    async fn unknown_callback_id_is_empty_success() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(Arc::new(HookRegistry::default()), None, tx);
        dispatcher.handle(hook_request("hook_404")).await;
        let line = rx.recv().await.expect("a response line");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        assert_eq!(value["response"]["subtype"], "success");
        assert_eq!(value["response"]["response"], serde_json::json!({}));
    }

    #[tokio::test]
    async fn hook_error_becomes_error_response() {
        let mut reg = HookRegistry::default();
        reg.register(HookEvent::Stop, None, Arc::new(FailingHook));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(Arc::new(reg), None, tx);
        dispatcher.handle(hook_request("hook_0")).await;
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
}
