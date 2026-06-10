//! The message stream type and its channel-backed producer.
//!
//! [`MessageStream`] is the boxed `Stream` every session surface returns. It is
//! produced from a `tokio` mpsc receiver by `ReceiverStream`, a thin adapter
//! that bridges `Receiver::poll_recv` to the `futures_core::Stream` contract so
//! no external stream-adapter crate is needed.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use tokio::sync::mpsc::Receiver;

use crate::agent::error::AgentError;
use crate::agent::message::Message;

/// A pinned, boxed stream of message frames (or per-item errors).
///
/// Returned by `query` and the `Runtime`/`Client` run methods. The stream ends
/// when the producing side closes its channel (e.g. after a `Result` frame).
pub type MessageStream = Pin<Box<dyn Stream<Item = Result<Message, AgentError>> + Send>>;

/// Adapts a `tokio` mpsc receiver into a [`MessageStream`].
pub(crate) struct ReceiverStream {
    rx: Receiver<Result<Message, AgentError>>,
}

impl ReceiverStream {
    /// Wrap a receiver as a stream source.
    pub(crate) const fn new(rx: Receiver<Result<Message, AgentError>>) -> Self {
        Self { rx }
    }

    /// Box and pin into the public [`MessageStream`] shape.
    pub(crate) fn boxed(self) -> MessageStream {
        Box::pin(self)
    }
}

impl Stream for ReceiverStream {
    type Item = Result<Message, AgentError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::ReceiverStream;
    use crate::agent::message::{Message, ResultMessage};
    use futures_util::StreamExt;
    use tokio::sync::mpsc;

    fn result_msg() -> Message {
        Message::Result(ResultMessage {
            result: "ok".into(),
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: crate::agent::types::SessionId::new("s1"),
            num_turns: 1,
        })
    }

    #[tokio::test]
    async fn forwards_items_then_ends_when_sender_drops() {
        let (tx, rx) = mpsc::channel(4);
        tx.send(Ok(result_msg())).await.expect("send");
        drop(tx);
        let mut stream = ReceiverStream::new(rx).boxed();
        let first = stream.next().await.expect("one item");
        assert!(matches!(first, Ok(Message::Result(_))));
        assert!(stream.next().await.is_none(), "ends after sender drops");
    }
}
