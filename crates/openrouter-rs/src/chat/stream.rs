//! Asynchronous SSE stream of [`StreamChunk`] items for a streaming chat
//! completion.
//!
//! Exists as its own file, gated behind `streaming`, so the
//! `eventsource-stream` dependency and the SSE driving logic stay off the
//! non-streaming surface. Wraps the raw response body as a
//! `Stream<Item = Result<StreamChunk, Error>>`.
//!
//! Responsibilities:
//! - Define [`ChatStream`], the body-stream wrapper that parses each SSE
//!   `data:` line into a [`StreamChunk`], terminates on `data: [DONE]`, and is
//!   terminal once it yields an error.
//!
//! Not responsible for:
//! - Building the HTTP request or checking the response status — that lives in
//!   `resource.rs` (`ChatResource::stream`), which only constructs a
//!   `ChatStream` after confirming a 2xx response.

use std::pin::Pin;
use std::task::{Context, Poll};

use eventsource_stream::{Event, Eventsource};
use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::chat::stream_chunk::StreamChunk;
use crate::error::Error;
use crate::transport::BodyStream;

/// Sentinel `data:` payload that terminates an OpenRouter SSE stream.
const DONE_SENTINEL: &str = "[DONE]";

pin_project! {
    /// Asynchronous stream of [`StreamChunk`] items for a streaming chat
    /// completion.
    ///
    /// Obtain via [`crate::chat::ChatResource::stream`].
    ///
    /// The stream is **terminal on error**: once it yields an
    /// `Err(Error::Stream(..))` (transport interruption or a mid-stream error
    /// event) or an `Err(Error::Serde(..))` (an undecodable chunk), the next
    /// poll returns `None`. A `data: [DONE]` line ends the stream cleanly.
    #[must_use = "streams do nothing unless polled"]
    pub struct ChatStream {
        #[pin]
        inner: eventsource_stream::EventStream<BodyStream>,
        terminated: bool,
    }
}

impl std::fmt::Debug for ChatStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatStream")
            .field("terminated", &self.terminated)
            .finish_non_exhaustive()
    }
}

impl ChatStream {
    /// Wrap a raw response body as a `ChatStream`.
    pub(crate) fn new(body: BodyStream) -> Self {
        Self {
            inner: body.eventsource(),
            terminated: false,
        }
    }
}

impl Stream for ChatStream {
    type Item = Result<StreamChunk, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if *this.terminated {
            return Poll::Ready(None);
        }

        match this.inner.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                *this.terminated = true;
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(e))) => {
                *this.terminated = true;
                Poll::Ready(Some(Err(Error::Stream(e.to_string()))))
            }
            Poll::Ready(Some(Ok(event))) => {
                if event.data == DONE_SENTINEL {
                    *this.terminated = true;
                    return Poll::Ready(None);
                }
                match parse_chunk(&event) {
                    Ok(chunk) => {
                        if let Some(err) = &chunk.error {
                            *this.terminated = true;
                            Poll::Ready(Some(Err(Error::Stream(err.message.clone()))))
                        } else {
                            Poll::Ready(Some(Ok(chunk)))
                        }
                    }
                    Err(e) => {
                        *this.terminated = true;
                        Poll::Ready(Some(Err(e)))
                    }
                }
            }
        }
    }
}

/// Parse one SSE event's `data` payload into a [`StreamChunk`].
fn parse_chunk(event: &Event) -> Result<StreamChunk, Error> {
    serde_json::from_str::<StreamChunk>(&event.data).map_err(|e| Error::Serde {
        context: "StreamChunk",
        source: e,
    })
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::panic,
        reason = "test-only panics on wrong-variant matches; a panic is the intended failure signal"
    )]

    use std::collections::VecDeque;

    use bytes::Bytes;
    use eventsource_stream::Event;
    use futures_core::Stream as FutStream;

    use super::*;
    use crate::error::TransportError;

    fn make_event(data: &str) -> Event {
        Event {
            data: data.to_owned(),
            ..Event::default()
        }
    }

    /// Single-shot in-memory byte stream for unit tests.
    struct ByteChunk(VecDeque<Bytes>);

    impl FutStream for ByteChunk {
        type Item = Result<Bytes, TransportError>;
        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.0.pop_front().map(Ok))
        }
    }

    fn body_from(s: &str) -> BodyStream {
        Box::pin(ByteChunk(VecDeque::from([Bytes::copy_from_slice(
            s.as_bytes(),
        )])))
    }

    fn token_chunk(content: &str) -> String {
        format!(
            "data: {{\"id\":\"g\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"x/y\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"{content}\"}},\"finish_reason\":null}}]}}\n\n"
        )
    }

    #[test]
    fn parse_chunk_decodes_data_payload() {
        let ev = make_event(
            r#"{"id":"g","object":"chat.completion.chunk","created":1,"model":"x/y","choices":[{"index":0,"delta":{"content":"hi"}}]}"#,
        );
        let chunk = parse_chunk(&ev).unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("hi"));
    }

    #[test]
    fn parse_chunk_rejects_malformed_json() {
        let ev = make_event("not json");
        assert!(matches!(parse_chunk(&ev), Err(Error::Serde { .. })));
    }

    #[tokio::test]
    async fn yields_chunks_then_terminates_on_done() {
        let sse = format!(
            "{}{}data: [DONE]\n\n",
            token_chunk("Hel"),
            token_chunk("lo")
        );
        let mut stream = ChatStream::new(body_from(&sse));

        let mut content = String::new();
        let mut count = 0;
        loop {
            let item = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
            match item {
                None => break,
                Some(Ok(chunk)) => {
                    count += 1;
                    if let Some(c) = &chunk.choices[0].delta.content {
                        content.push_str(c);
                    }
                }
                Some(Err(e)) => panic!("unexpected error: {e:?}"),
            }
        }
        assert_eq!(count, 2);
        assert_eq!(content, "Hello");
    }

    #[tokio::test]
    async fn mid_stream_error_yields_stream_error_then_terminates() {
        let sse = concat!(
            "data: {\"id\":\"g\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"x/y\",",
            "\"error\":{\"code\":\"server_error\",\"message\":\"provider disconnected\"},",
            "\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"error\"}]}\n\n",
        );
        let mut stream = ChatStream::new(body_from(sse));

        let first = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        match first {
            Some(Err(Error::Stream(msg))) => assert_eq!(msg, "provider disconnected"),
            other => panic!("expected Error::Stream, got {other:?}"),
        }

        let second = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        assert!(second.is_none(), "stream must terminate after error chunk");
    }

    #[tokio::test]
    async fn malformed_chunk_yields_serde_error_then_terminates() {
        let sse = "data: {not valid json}\n\n";
        let mut stream = ChatStream::new(body_from(sse));

        let first = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        assert!(
            matches!(first, Some(Err(Error::Serde { .. }))),
            "expected Error::Serde, got {first:?}"
        );

        let second = std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)).await;
        assert!(
            second.is_none(),
            "stream must terminate after a decode error"
        );
    }
}
