//! A stream adapter that observes each item, then forwards it unchanged.
//!
//! The shared primitive behind the observing layers: it runs a closure by
//! shared reference on every polled item — including error items — and yields
//! the item without cloning or reordering. It ends exactly when its inner
//! stream ends.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use crate::agent::error::AgentError;
use crate::agent::message::Message;

pin_project_lite::pin_project! {
    /// Wraps a message stream, running `f` on each item before forwarding it.
    pub(crate) struct Tap<S, F> {
        #[pin]
        inner: S,
        f: F,
    }
}

impl<S, F> Tap<S, F>
where
    S: Stream<Item = Result<Message, AgentError>>,
    F: FnMut(&Result<Message, AgentError>),
{
    /// Wrap `inner`, observing each item with `f`.
    pub(crate) const fn new(inner: S, f: F) -> Self {
        Self { inner, f }
    }
}

impl<S, F> Stream for Tap<S, F>
where
    S: Stream<Item = Result<Message, AgentError>>,
    F: FnMut(&Result<Message, AgentError>),
{
    type Item = Result<Message, AgentError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Ready(Some(item)) => {
                (this.f)(&item);
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::Tap;
    use crate::agent::error::AgentError;
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::types::SessionId;
    use futures_util::{StreamExt, stream};
    use std::sync::{Arc, Mutex};

    fn result(text: &str) -> Message {
        Message::Result(ResultMessage {
            result: text.into(),
            structured_output: None,
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })
    }

    #[tokio::test]
    async fn observes_each_item_and_forwards_unchanged() {
        let items = vec![
            Ok(result("a")),
            Err(AgentError::TransportClosed),
            Ok(result("b")),
        ];
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_w = Arc::clone(&seen);
        let tapped = Tap::new(
            stream::iter(items),
            move |item: &Result<Message, AgentError>| {
                seen_w.lock().expect("lock").push(item.is_ok());
            },
        );
        let out: Vec<_> = tapped.collect().await;
        assert_eq!(out.len(), 3, "forwards every item");
        assert!(matches!(out[0], Ok(Message::Result(_))));
        assert!(matches!(out[1], Err(AgentError::TransportClosed)));
        assert!(matches!(out[2], Ok(Message::Result(_))));
        assert_eq!(
            *seen.lock().expect("lock"),
            vec![true, false, true],
            "sees items in order"
        );
    }
}
