//! Demultiplexing decoded frames to the active turn or a pending waiter.
//!
//! The reader task decodes each stdout line into an [`InboundFrame`] and hands
//! it here. Message frames go to the current turn's channel (cleared when the
//! terminal `Result` frame arrives); control responses resolve the matching
//! pending request by id. An unexpected inbound control request is surfaced on
//! the active turn as a protocol error, since no handler is registered yet.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use tokio::sync::{mpsc, oneshot};

use crate::agent::error::AgentError;
use crate::agent::message::Message;
use crate::agent::protocol::{ControlResponseBody, InboundFrame};

/// Routes inbound frames to the active turn stream and pending control waiters.
pub(super) struct Demux {
    pending: Mutex<HashMap<String, oneshot::Sender<ControlResponseBody>>>,
    turn_sink: Mutex<Option<mpsc::Sender<Result<Message, AgentError>>>>,
}

impl Demux {
    /// Create an empty demultiplexer.
    pub(super) fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            turn_sink: Mutex::new(None),
        }
    }

    /// Install the message sink for the turn that is about to start.
    pub(super) fn set_turn_sink(&self, sink: mpsc::Sender<Result<Message, AgentError>>) {
        *self
            .turn_sink
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(sink);
    }

    /// Register a waiter for the control response correlated to `id`.
    pub(super) fn register_pending(
        &self,
        id: String,
        waiter: oneshot::Sender<ControlResponseBody>,
    ) {
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, waiter);
    }

    /// Drop the pending waiter for `id` (e.g. when its request could not be sent).
    pub(super) fn remove_pending(&self, id: &str) {
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(id);
    }

    /// Clone the current turn sink out from under the lock (never held across `.await`).
    fn take_sink_handle(&self) -> Option<mpsc::Sender<Result<Message, AgentError>>> {
        self.turn_sink
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn clear_sink(&self) {
        *self
            .turn_sink
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// Route one decoded frame to its destination.
    pub(super) async fn route(&self, frame: InboundFrame) {
        match frame {
            InboundFrame::Message(message) => {
                let is_result = matches!(message, Message::Result(_));
                if let Some(sink) = self.take_sink_handle() {
                    let _ = sink.send(Ok(message)).await;
                }
                if is_result {
                    self.clear_sink();
                }
            }
            InboundFrame::ControlResponse(response) => {
                let body = response.response;
                let id = body.request_id().to_string();
                // Extract the waiter before the await to avoid holding the guard.
                let waiter = self
                    .pending
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(&id);
                if let Some(waiter) = waiter {
                    let _ = waiter.send(body);
                }
            }
            InboundFrame::ControlRequest(_) => {
                // No hook or permission handler is registered, so the backend
                // does not issue these. Surface any that arrives rather than
                // dropping it silently or deadlocking the backend.
                self.fail_turn(AgentError::Protocol {
                    detail: "received an inbound control request with no handler registered"
                        .to_string(),
                })
                .await;
            }
        }
    }

    /// Forward an error item onto the active turn, if any.
    pub(super) async fn fail_turn(&self, error: AgentError) {
        if let Some(sink) = self.take_sink_handle() {
            let _ = sink.send(Err(error)).await;
        }
    }

    /// Signal the active turn that the transport closed, then clear it.
    pub(super) async fn close(&self) {
        self.fail_turn(AgentError::TransportClosed).await;
        self.clear_sink();
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::Demux;
    use crate::agent::message::Message;
    use crate::agent::protocol::decode_inbound;
    use tokio::sync::{mpsc, oneshot};

    #[tokio::test]
    async fn routes_message_frame_to_turn_sink_and_clears_on_result() {
        let demux = Demux::new();
        let (tx, mut rx) = mpsc::channel(4);
        demux.set_turn_sink(tx);
        let frame = decode_inbound(
            r#"{"type":"result","subtype":"success","result":"ok","is_error":false,"session_id":"s1","num_turns":1}"#,
        )
        .expect("decode");
        demux.route(frame).await;
        let got = rx.recv().await.expect("message");
        assert!(matches!(got, Ok(Message::Result(_))));
        // Result frame clears the sink: the channel is now closed.
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn routes_control_response_to_pending_waiter() {
        let demux = Demux::new();
        let (tx, rx) = oneshot::channel();
        demux.register_pending("req_1".to_string(), tx);
        let frame = decode_inbound(
            r#"{"type":"control_response","response":{"subtype":"success","request_id":"req_1","response":{"ok":true}}}"#,
        )
        .expect("decode");
        demux.route(frame).await;
        let body = rx.await.expect("resolved");
        assert_eq!(body.request_id(), "req_1");
    }

    #[tokio::test]
    async fn fail_turn_forwards_error_item() {
        let demux = Demux::new();
        let (tx, mut rx) = mpsc::channel(1);
        demux.set_turn_sink(tx);
        demux
            .fail_turn(crate::agent::error::AgentError::TransportClosed)
            .await;
        assert!(matches!(rx.recv().await, Some(Err(_))));
    }

    #[tokio::test]
    async fn remove_pending_drops_waiter_before_late_route() {
        let demux = Demux::new();
        let (tx, rx) = oneshot::channel();
        demux.register_pending("req_x".to_string(), tx);

        // Remove the pending entry as if the write failed after registration.
        demux.remove_pending("req_x");

        // A late response for the same id must NOT resolve the original receiver.
        let frame = decode_inbound(
            r#"{"type":"control_response","response":{"subtype":"success","request_id":"req_x","response":{"ok":true}}}"#,
        )
        .expect("decode");
        demux.route(frame).await;

        // The sender was dropped by remove_pending, so the receiver must be Err.
        assert!(
            rx.await.is_err(),
            "removed waiter should not be resolved by the late response"
        );
    }
}
