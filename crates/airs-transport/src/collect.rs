//! Drain an incremental [`crate::BodyStream`] into a byte buffer with a cap.
//!
//! Generic over provider — operates purely on bytes and a size limit.

use crate::BodyStream;
use crate::error::TransportError;

/// Maximum response body size accepted before truncation.
///
/// 16 MiB is a conservative ceiling well above any plausible non-streaming
/// API response.
pub const MAX_RESPONSE_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Collect a [`BodyStream`] into a byte buffer, stopping at `limit` bytes.
///
/// # Errors
/// Returns [`TransportError::BodyStream`] if the stream yields an error or if
/// the accumulated size exceeds `limit`.
pub async fn collect_body(mut stream: BodyStream, limit: usize) -> Result<Vec<u8>, TransportError> {
    let mut buf = Vec::new();
    loop {
        let item = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await;
        match item {
            None => break,
            Some(Err(e)) => return Err(e),
            Some(Ok(chunk)) => {
                if buf.len() + chunk.len() > limit {
                    return Err(TransportError::BodyStream(format!(
                        "response body exceeded {limit} byte limit"
                    )));
                }
                buf.extend_from_slice(&chunk);
            }
        }
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use bytes::Bytes;
    use futures_core::Stream;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    fn body_from(payload: &'static [u8]) -> BodyStream {
        struct Once(Option<Bytes>);
        impl Stream for Once {
            type Item = Result<Bytes, TransportError>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                Poll::Ready(self.0.take().map(Ok))
            }
        }
        Box::pin(Once(Some(Bytes::from_static(payload))))
    }

    #[tokio::test]
    async fn collect_body_drains_within_limit() {
        let bytes = collect_body(body_from(b"hello world"), 1024).await.unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[tokio::test]
    async fn collect_body_rejects_over_limit() {
        let err = collect_body(body_from(b"too big"), 3).await.unwrap_err();
        assert!(matches!(err, TransportError::BodyStream(_)));
    }
}
