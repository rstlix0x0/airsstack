//! JSONL batch results stream.
//!
//! Exists as its own module to keep the line-splitting and stream-driving
//! logic separate from the HTTP dispatch in `resource.rs`.
//!
//! Responsibilities:
//! - Define [`BatchResultStream`], an async iterator over decoded
//!   [`BatchResultRow`] values from a `GET /v1/messages/batches/{id}/results`
//!   response body.
//! - Implement the line-splitting buffer on top of a raw [`BodyStream`].
//!
//! Not responsible for:
//! - Fetching the HTTP response — that is the resource layer.
//! - Authenticating or configuring the request — the client layer handles those.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::error::Error;
use crate::messages::batches::types::BatchResultRow;
use crate::transport::BodyStream;

pin_project! {
    /// Async iterator over `Result<BatchResultRow, Error>`.
    ///
    /// Each item is one JSONL line decoded from the body of
    /// `GET /v1/messages/batches/{id}/results`. Poll with
    /// `StreamExt::next` from `futures_util` or the equivalent
    /// `std::future::poll_fn` adapter.
    ///
    /// Yields [`Error::JsonLines`] when a line cannot be decoded.
    /// Yields [`Error::Transport`] when the underlying body stream errors.
    /// After any error the stream terminates.
    #[must_use = "streams do nothing unless polled"]
    pub struct BatchResultStream {
        #[pin]
        body: BodyStream,
        buf: BytesMut,
        terminated: bool,
    }
}

impl std::fmt::Debug for BatchResultStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchResultStream")
            .field("terminated", &self.terminated)
            .field("buf_len", &self.buf.len())
            .finish_non_exhaustive()
    }
}

impl BatchResultStream {
    /// Wrap a raw body stream as a typed JSONL result stream.
    pub(crate) fn new(body: BodyStream) -> Self {
        Self {
            body,
            buf: BytesMut::with_capacity(8 * 1024),
            terminated: false,
        }
    }

    /// Split the next newline-terminated line out of the buffer.
    ///
    /// Returns `None` when no newline is present in the current buffer
    /// contents. The returned bytes do not include the newline.
    fn try_split_line(buf: &mut BytesMut) -> Option<Bytes> {
        let pos = buf.iter().position(|&b| b == b'\n')?;
        let line = buf.split_to(pos).freeze();
        // Discard the newline byte.
        let _ = buf.split_to(1);
        Some(line)
    }
}

impl Stream for BatchResultStream {
    type Item = Result<BatchResultRow, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        if *this.terminated {
            return Poll::Ready(None);
        }

        loop {
            if let Some(line) = Self::try_split_line(this.buf) {
                if line.is_empty() {
                    continue;
                }
                let parsed = serde_json::from_slice::<BatchResultRow>(&line)
                    .map_err(|e| Error::JsonLines(format!("decode row: {e}")));
                return Poll::Ready(Some(parsed));
            }

            match this.body.as_mut().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    *this.terminated = true;
                    if !this.buf.is_empty() {
                        let tail = this.buf.split().freeze();
                        let parsed = serde_json::from_slice::<BatchResultRow>(&tail)
                            .map_err(|e| Error::JsonLines(format!("decode tail row: {e}")));
                        return Poll::Ready(Some(parsed));
                    }
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Err(e))) => {
                    *this.terminated = true;
                    return Poll::Ready(Some(Err(Error::Transport(e))));
                }
                Poll::Ready(Some(Ok(chunk))) => {
                    this.buf.extend_from_slice(&chunk);
                    // Loop again — try_split_line will see the new bytes.
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use bytes::Bytes;
    use futures_core::Stream;

    use super::*;
    use crate::error::TransportError;

    /// Build an in-memory `BodyStream` that yields the supplied byte slices
    /// one chunk at a time, in order, then signals end-of-stream.
    fn body_from_chunks(chunks: Vec<Bytes>) -> BodyStream {
        struct V(VecDeque<Bytes>);

        impl Stream for V {
            type Item = Result<Bytes, TransportError>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                Poll::Ready(self.0.pop_front().map(Ok))
            }
        }

        Box::pin(V(chunks.into_iter().collect()))
    }

    fn body_from_statics(chunks: Vec<&'static [u8]>) -> BodyStream {
        body_from_chunks(chunks.into_iter().map(Bytes::from_static).collect())
    }

    /// Drive the stream to completion, collecting all items.
    async fn collect_stream(mut s: BatchResultStream) -> Vec<Result<BatchResultRow, Error>> {
        let mut items = Vec::new();
        loop {
            let item = std::future::poll_fn(|cx| Pin::new(&mut s).poll_next(cx)).await;
            match item {
                Some(v) => items.push(v),
                None => break,
            }
        }
        items
    }

    #[tokio::test]
    async fn splits_three_rows() {
        let body = body_from_statics(vec![
            br#"{"custom_id":"r1","result":{"type":"canceled"}}"#,
            b"\n",
            br#"{"custom_id":"r2","result":{"type":"expired"}}"#,
            b"\n",
            br#"{"custom_id":"r3","result":{"type":"canceled"}}"#,
            b"\n",
        ]);
        let s = BatchResultStream::new(body);
        let items = collect_stream(s).await;
        assert_eq!(items.len(), 3);
        let ids: Vec<_> = items
            .into_iter()
            .map(|r| r.unwrap().custom_id.as_str().to_owned())
            .collect();
        assert_eq!(ids, vec!["r1", "r2", "r3"]);
    }

    #[tokio::test]
    async fn tail_without_trailing_newline_is_flushed() {
        // The last row has no trailing '\n'; the stream must still yield it.
        let body = body_from_statics(vec![
            br#"{"custom_id":"r1","result":{"type":"canceled"}}"#,
            b"\n",
            br#"{"custom_id":"r2","result":{"type":"expired"}}"#,
            // No trailing newline.
        ]);
        let s = BatchResultStream::new(body);
        let items = collect_stream(s).await;
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].as_ref().unwrap().custom_id.as_str(), "r2");
    }

    #[tokio::test]
    async fn multi_row_single_chunk() {
        // Two rows in one chunk separated by a newline.
        let combined: Bytes = Bytes::from(
            b"{\"custom_id\":\"r1\",\"result\":{\"type\":\"canceled\"}}\n{\"custom_id\":\"r2\",\"result\":{\"type\":\"expired\"}}\n"
                .to_vec(),
        );
        let body = body_from_chunks(vec![combined]);
        let s = BatchResultStream::new(body);
        let items = collect_stream(s).await;
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn blank_lines_are_skipped() {
        let body = body_from_statics(vec![
            br#"{"custom_id":"r1","result":{"type":"canceled"}}"#,
            b"\n\n",
            br#"{"custom_id":"r2","result":{"type":"expired"}}"#,
            b"\n",
        ]);
        let s = BatchResultStream::new(body);
        let items = collect_stream(s).await;
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn transport_error_yields_one_err_then_terminates() {
        struct ErrorAfterOne {
            chunks: VecDeque<Bytes>,
            emitted: bool,
        }

        impl Stream for ErrorAfterOne {
            type Item = Result<Bytes, TransportError>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                if let Some(chunk) = self.chunks.pop_front() {
                    return Poll::Ready(Some(Ok(chunk)));
                }
                if !self.emitted {
                    self.emitted = true;
                    return Poll::Ready(Some(Err(TransportError::BodyStream(
                        "simulated error".into(),
                    ))));
                }
                Poll::Ready(None)
            }
        }

        let body: BodyStream = Box::pin(ErrorAfterOne {
            chunks: std::iter::once(Bytes::from_static(
                b"{\"custom_id\":\"r1\",\"result\":{\"type\":\"canceled\"}}\n",
            ))
            .collect(),
            emitted: false,
        });

        let s = BatchResultStream::new(body);
        let items = collect_stream(s).await;
        // First item: successful row
        assert!(items[0].is_ok());
        // Second item: transport error
        assert!(matches!(items[1], Err(Error::Transport(_))));
        // No further items
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn invalid_json_line_yields_json_lines_error() {
        let body = body_from_statics(vec![b"this is not json\n"]);
        let s = BatchResultStream::new(body);
        let items = collect_stream(s).await;
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], Err(Error::JsonLines(_))));
    }
}
