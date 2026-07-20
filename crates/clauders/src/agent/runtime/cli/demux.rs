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
    // `None` means the transport is closed. Folding the closed state into the
    // same `Option` the map lives in (rather than a separate `AtomicBool`)
    // makes "closed" and "the map is gone" the same fact behind the same
    // lock: there is no window in which one is true and the other isn't, so
    // the race `register_pending` and `close()` used to negotiate via lock
    // ordering cannot exist in the first place.
    pending: Mutex<Option<HashMap<String, oneshot::Sender<ControlResponseBody>>>>,
    turn_sink: Mutex<Option<mpsc::Sender<Result<Message, AgentError>>>>,
}

impl Demux {
    /// Create an empty demultiplexer.
    pub(super) fn new() -> Self {
        Self {
            pending: Mutex::new(Some(HashMap::new())),
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
    ///
    /// # Errors
    /// Returns [`AgentError::TransportClosed`] without registering `waiter`
    /// if [`Demux::close`] has already run. Once the transport is closed,
    /// nothing will ever remove or resolve a freshly inserted waiter, so
    /// registering one anyway would leave its caller waiting out the full
    /// control-request timeout for an answer that can never come.
    pub(super) fn register_pending(
        &self,
        id: String,
        waiter: oneshot::Sender<ControlResponseBody>,
    ) -> Result<(), AgentError> {
        // The map and the closed state are the same `Option`, guarded by the
        // same lock `close()` takes to drain: either this call's insert lands
        // while the map is still `Some` (and gets drained by a `close()` that
        // has not yet run, or lives on until one does), or `close()` has
        // already replaced it with `None` and the match below refuses the
        // insert. There is no interleaving that inserts a waiter after the
        // drain has already run, because "drained" and "closed" cannot
        // disagree — they are one flag, not two.
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
            .map_or(Err(AgentError::TransportClosed), |pending| {
                pending.insert(id, waiter);
                Ok(())
            })
    }

    /// Drop the pending waiter for `id` (e.g. when its request could not be sent).
    ///
    /// A no-op if the transport is already closed: `close()` has already
    /// drained (and dropped) every waiter, so there is nothing left to remove.
    pub(super) fn remove_pending(&self, id: &str) {
        if let Some(pending) = &mut *self.pending.lock().unwrap_or_else(PoisonError::into_inner) {
            pending.remove(id);
        }
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
                // A response arriving after `close()` finds `None` here — the
                // map (and every waiter in it) is already gone — and is
                // dropped as a no-op rather than resolving anything.
                let waiter = self
                    .pending
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .as_mut()
                    .and_then(|pending| pending.remove(&id));
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

    /// Signal the active turn and every pending control waiter that the
    /// transport closed.
    ///
    /// A closed transport (clean EOF, a crashed binary, a killed child) means
    /// no correlated `control_response` will ever arrive, so any waiter left
    /// in `pending` would hang its `send_control` caller forever. Taking the
    /// map and dropping each sender resolves every such waiter's `rx` with
    /// `Err(RecvError)`, which `send_control` already maps to
    /// [`AgentError::TransportClosed`] — no new error path is needed.
    ///
    /// The take leaves `None` behind, which is also the closed state
    /// `register_pending` checks: there is no separate flag to set, so
    /// closing is visible to `register_pending` atomically with the take —
    /// under the same lock acquisition, not a step before or after it. A
    /// registration racing this call either lands before the take (and gets
    /// taken and dropped with everything else) or sees `None` and is refused;
    /// there is no third outcome.
    ///
    /// The take-and-drop runs first and unconditionally, before `fail_turn`:
    /// the turn sink is a bounded channel, so `fail_turn`'s send can park if
    /// the caller isn't draining its `MessageStream`. Pending waiters must
    /// not wait on that.
    pub(super) async fn close(&self) {
        // `drop(...)` states the drop point explicitly: the taken map (and
        // every sender in it) must go before the `fail_turn` await below, so
        // every pending waiter is resolved even if `fail_turn`'s send parks
        // on a full, undrained turn sink. Binding the take to a `let` instead
        // would silently defer the drop to end of scope and reintroduce that
        // hazard.
        drop(
            self.pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take(),
        );
        self.fail_turn(AgentError::TransportClosed).await;
        self.clear_sink();
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::sync::Arc;

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
        demux
            .register_pending("req_1".to_string(), tx)
            .expect("register before close succeeds");
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
    async fn routes_malformed_control_response_to_pending_waiter_instead_of_hanging() {
        // A control_response with a subtype this version does not model, but
        // a recoverable request_id, must still resolve the waiter — the
        // outbound mirror of the inbound control_request rescue. A bounded
        // timeout guards this specific assertion (rather than a bare await)
        // because a regression here reproduces as an indefinite hang, not a
        // clean test failure.
        let demux = Demux::new();
        let (tx, rx) = oneshot::channel();
        demux
            .register_pending("req_9".to_string(), tx)
            .expect("register before close succeeds");
        let frame = decode_inbound(
            r#"{"type":"control_response","response":{"subtype":"some_future_subtype","request_id":"req_9"}}"#,
        )
        .expect("decode must not fail");
        demux.route(frame).await;
        let body = tokio::time::timeout(std::time::Duration::from_secs(1), rx)
            .await
            .expect("waiter resolved within the timeout instead of hanging")
            .expect("resolved to a value, not a dropped sender");
        assert_eq!(body.request_id(), "req_9");
        assert!(
            matches!(
                body,
                crate::agent::protocol::ControlResponseBody::Malformed { .. }
            ),
            "expected Malformed, got {body:?}"
        );
    }

    #[tokio::test]
    async fn close_resolves_pending_waiters_instead_of_hanging() {
        // A transport close (stdout EOF, a crashed binary, a killed child)
        // must not leave a `send_control` caller parked forever: every
        // pending oneshot has to be signaled, not just the active turn.
        let demux = Demux::new();
        let (tx, rx) = oneshot::channel();
        demux
            .register_pending("req_close".to_string(), tx)
            .expect("register before close succeeds");

        demux.close().await;

        let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
        assert!(
            result.is_ok(),
            "pending waiter was not resolved within the timeout after close()"
        );
        assert!(
            result.expect("timeout already checked").is_err(),
            "expected the sender to be dropped (RecvError), not a value"
        );
    }

    #[tokio::test]
    async fn close_resolves_pending_waiter_even_when_turn_sink_is_full() {
        // `close()` signals the active turn via `fail_turn`, which does
        // `sink.send(...).await` on the bounded (64-capacity) turn channel.
        // If that channel is full and nobody is draining it — a caller
        // holding a `MessageStream` it never polls — the send parks forever.
        // A pending control waiter must still resolve regardless: draining
        // `pending` cannot be gated on what happens to the turn sink.
        let demux = Arc::new(Demux::new());
        let (tx, _rx) = mpsc::channel(64);
        for _ in 0..64 {
            tx.try_send(Ok(Message::Other(serde_json::json!({}))))
                .expect("channel has capacity for 64 items");
        }
        demux.set_turn_sink(tx);

        let (ptx, prx) = oneshot::channel();
        demux
            .register_pending("req_full".to_string(), ptx)
            .expect("register before close succeeds");

        // Run close() in the background: it may itself park indefinitely on
        // the full, undrained turn sink, which is orthogonal to whether the
        // pending waiter gets resolved.
        let demux_bg = Arc::clone(&demux);
        tokio::spawn(async move { demux_bg.close().await });

        let result = tokio::time::timeout(std::time::Duration::from_secs(1), prx).await;
        assert!(
            result.is_ok(),
            "pending waiter was not resolved within the timeout after close(), \
             even though the turn sink was full and undrained"
        );
        assert!(
            result.expect("timeout already checked").is_err(),
            "expected the sender to be dropped (RecvError), not a value"
        );
    }

    #[tokio::test]
    async fn register_pending_after_close_fails_fast_with_transport_closed() {
        // Once `close()` has drained `pending`, nothing will ever remove or
        // resolve a fresh waiter — registering one anyway would leave its
        // caller waiting out the full control-request timeout for an answer
        // that can never arrive. `register_pending` must refuse immediately
        // instead, with no timeout involved at all.
        let demux = Demux::new();
        demux.close().await;

        let (tx, _rx) = oneshot::channel();
        let result = demux.register_pending("req_late".to_string(), tx);

        assert!(
            matches!(
                result,
                Err(crate::agent::error::AgentError::TransportClosed)
            ),
            "expected TransportClosed, got {result:?}"
        );
    }

    #[tokio::test]
    async fn remove_pending_and_late_route_are_no_ops_after_close() {
        // Once `close()` has taken the map, both `remove_pending` and a
        // routed control response must treat "nothing to remove" and
        // "nothing to resolve" as ordinary no-ops rather than panicking on
        // a missing map.
        let demux = Demux::new();
        demux.close().await;

        demux.remove_pending("req_gone");

        let frame = decode_inbound(
            r#"{"type":"control_response","response":{"subtype":"success","request_id":"req_gone","response":{"ok":true}}}"#,
        )
        .expect("decode");
        demux.route(frame).await;
    }

    #[tokio::test]
    async fn remove_pending_drops_waiter_before_late_route() {
        let demux = Demux::new();
        let (tx, rx) = oneshot::channel();
        demux
            .register_pending("req_x".to_string(), tx)
            .expect("register before close succeeds");

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
