use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};

/// Upper bound on retained stderr bytes (last N kept).
const STDERR_CAP: usize = 64 * 1024;

/// Per-chunk stderr callback: invoked with one valid UTF-8 chunk at a time.
type StderrCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Append `data` to `buf`, keeping at most the last `cap` bytes.
fn append_bounded(buf: &mut Vec<u8>, data: &[u8], cap: usize) {
    buf.extend_from_slice(data);
    let overflow = buf.len().saturating_sub(cap);
    if overflow > 0 {
        buf.drain(0..overflow);
    }
}

/// Failure reading a line from the child's stdout.
#[derive(Debug)]
pub enum LineError {
    /// Underlying I/O error (or invalid UTF-8 in a line).
    Io(std::io::Error),
    /// A single line exceeded the configured `max_buffer_size` cap.
    Overflow {
        /// The configured byte cap that was exceeded.
        cap: usize,
    },
}

/// Read one newline-terminated line into `buf` (without the newline),
/// bounding the accumulated bytes to `cap` when set.
///
/// Returns `Ok(true)` when a line was read, `Ok(false)` at EOF with no
/// pending bytes, or `Err(LineError::Overflow)` when the line exceeds `cap`
/// before a newline arrives.
async fn read_capped_line<R>(
    reader: &mut R,
    cap: Option<usize>,
    buf: &mut Vec<u8>,
) -> Result<bool, LineError>
where
    R: AsyncBufRead + Unpin,
{
    buf.clear();
    loop {
        let chunk = reader.fill_buf().await.map_err(LineError::Io)?;
        if chunk.is_empty() {
            // EOF: an unterminated trailing line still counts as a line.
            return Ok(!buf.is_empty());
        }
        if let Some(pos) = chunk.iter().position(|b| *b == b'\n') {
            buf.extend_from_slice(&chunk[..pos]);
            Pin::new(&mut *reader).consume(pos + 1);
            if let Some(cap) = cap
                && buf.len() > cap
            {
                return Err(LineError::Overflow { cap });
            }
            return Ok(true);
        }
        buf.extend_from_slice(chunk);
        let advance = chunk.len();
        Pin::new(&mut *reader).consume(advance);
        if let Some(cap) = cap
            && buf.len() > cap
        {
            return Err(LineError::Overflow { cap });
        }
    }
}

/// Line-oriented, optionally byte-capped view over a child's stdout.
pub struct StdoutLines {
    reader: BufReader<ChildStdout>,
    cap: Option<usize>,
    buf: Vec<u8>,
}

impl StdoutLines {
    pub(crate) fn new(stdout: ChildStdout, cap: Option<NonZeroUsize>) -> Self {
        Self {
            reader: BufReader::new(stdout),
            cap: cap.map(NonZeroUsize::get),
            buf: Vec::new(),
        }
    }

    /// Read the next line (without the trailing newline), or `None` at EOF.
    ///
    /// # Errors
    /// Returns [`LineError::Io`] on an I/O failure or invalid UTF-8, or
    /// [`LineError::Overflow`] when a line exceeds the configured cap.
    pub async fn next_line(&mut self) -> Result<Option<String>, LineError> {
        if read_capped_line(&mut self.reader, self.cap, &mut self.buf).await? {
            let mut bytes = std::mem::take(&mut self.buf);
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            let line = String::from_utf8(bytes).map_err(|e| {
                LineError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            })?;
            Ok(Some(line))
        } else {
            Ok(None)
        }
    }
}

/// Incrementally decodes a byte stream to UTF-8, holding back a partial
/// trailing multibyte sequence until the next chunk completes it.
#[derive(Default)]
struct IncrementalUtf8 {
    carry: Vec<u8>,
}

impl IncrementalUtf8 {
    /// Feed the next raw chunk; return the text safely decodable so far.
    fn push(&mut self, data: &[u8]) -> String {
        let mut bytes = std::mem::take(&mut self.carry);
        bytes.extend_from_slice(data);
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_owned(),
            Err(e) => {
                let valid = e.valid_up_to();
                let mut out = String::from_utf8_lossy(&bytes[..valid]).into_owned();
                match e.error_len() {
                    None => {
                        // Incomplete trailing sequence — carry to the next push.
                        self.carry = bytes[valid..].to_vec();
                    }
                    Some(bad) => {
                        // Genuinely invalid bytes — emit a replacement, skip them.
                        out.push('\u{FFFD}');
                        self.carry = bytes[valid + bad..].to_vec();
                    }
                }
                out
            }
        }
    }

    /// Flush any remaining bytes at EOF (lossily).
    fn finish(&mut self) -> String {
        if self.carry.is_empty() {
            String::new()
        } else {
            String::from_utf8_lossy(&std::mem::take(&mut self.carry)).into_owned()
        }
    }
}

/// A bounded, continuously-drained snapshot of a child's stderr.
///
/// A background task reads stderr to EOF so the child can never block on a
/// full stderr pipe; only the most recent 64 KiB are retained.
#[derive(Clone)]
pub struct StderrBuffer {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl StderrBuffer {
    /// Spawn the drain task and return a handle to the captured bytes.
    ///
    /// When `on_chunk` is set, each decoded stderr chunk is also handed to it
    /// as a valid `&str` (incremental UTF-8 boundary decode); the bounded
    /// snapshot buffer is filled regardless (the callback augments, never
    /// suppresses).
    pub(crate) fn drain(mut stderr: ChildStderr, on_chunk: Option<StderrCallback>) -> Self {
        let inner = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&inner);
        tokio::spawn(async move {
            let mut chunk = [0u8; 4096];
            let mut decoder = IncrementalUtf8::default();
            loop {
                match stderr.read(&mut chunk).await {
                    Ok(0) | Err(_) => {
                        if let Some(cb) = &on_chunk {
                            let tail = decoder.finish();
                            if !tail.is_empty() {
                                cb(&tail);
                            }
                        }
                        break;
                    }
                    Ok(n) => {
                        if let Ok(mut guard) = sink.lock() {
                            append_bounded(&mut guard, &chunk[..n], STDERR_CAP);
                        }
                        if let Some(cb) = &on_chunk {
                            let text = decoder.push(&chunk[..n]);
                            if !text.is_empty() {
                                cb(&text);
                            }
                        }
                    }
                }
            }
        });
        Self { inner }
    }

    /// Current captured stderr as a lossy UTF-8 string.
    #[must_use]
    pub fn snapshot(&self) -> String {
        self.inner
            .lock()
            .map(|guard| String::from_utf8_lossy(&guard).into_owned())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::append_bounded;

    #[test]
    fn append_bounded_keeps_only_the_last_cap_bytes() {
        let mut buf = Vec::new();
        append_bounded(&mut buf, b"hello", 4);
        assert_eq!(buf, b"ello");

        append_bounded(&mut buf, b"XY", 4);
        assert_eq!(buf, b"loXY");

        append_bounded(&mut buf, b"", 4);
        assert_eq!(buf, b"loXY");
    }

    #[test]
    fn append_bounded_under_cap_appends_all() {
        let mut buf = Vec::new();
        append_bounded(&mut buf, b"ab", 8);
        append_bounded(&mut buf, b"cd", 8);
        assert_eq!(buf, b"abcd");
    }

    #[tokio::test]
    async fn read_capped_line_reads_lines_within_cap() {
        use tokio::io::BufReader;
        let data = b"hello\nworld\n";
        let mut r = BufReader::new(&data[..]);
        let mut buf = Vec::new();
        assert!(
            super::read_capped_line(&mut r, Some(10), &mut buf)
                .await
                .expect("ok")
        );
        assert_eq!(buf, b"hello");
        assert!(
            super::read_capped_line(&mut r, Some(10), &mut buf)
                .await
                .expect("ok")
        );
        assert_eq!(buf, b"world");
        assert!(
            !super::read_capped_line(&mut r, Some(10), &mut buf)
                .await
                .expect("ok")
        );
    }

    #[tokio::test]
    async fn read_capped_line_errors_when_a_line_exceeds_cap() {
        use tokio::io::BufReader;
        let data = b"this-line-is-far-too-long\n";
        let mut r = BufReader::new(&data[..]);
        let mut buf = Vec::new();
        let err = super::read_capped_line(&mut r, Some(8), &mut buf)
            .await
            .unwrap_err();
        assert!(matches!(err, super::LineError::Overflow { cap: 8 }));
    }

    #[tokio::test]
    async fn read_capped_line_unbounded_reads_any_length() {
        use tokio::io::BufReader;
        let big = format!("{}\n", "x".repeat(100_000));
        let mut r = BufReader::new(big.as_bytes());
        let mut buf = Vec::new();
        assert!(
            super::read_capped_line(&mut r, None, &mut buf)
                .await
                .expect("ok")
        );
        assert_eq!(buf.len(), 100_000);
    }

    #[test]
    fn incremental_utf8_reassembles_a_split_multibyte_char() {
        // "é" = 0xC3 0xA9 split across two pushes.
        let mut d = super::IncrementalUtf8::default();
        assert_eq!(d.push(&[0xC3]), "");
        assert_eq!(d.push(&[0xA9]), "é");
        assert_eq!(d.finish(), "");
    }

    #[test]
    fn incremental_utf8_passes_ascii_through() {
        let mut d = super::IncrementalUtf8::default();
        assert_eq!(d.push(b"hello"), "hello");
        assert_eq!(d.finish(), "");
    }

    #[test]
    fn incremental_utf8_flushes_dangling_lead_byte_lossily_at_eof() {
        let mut d = super::IncrementalUtf8::default();
        assert_eq!(d.push(&[0xC3]), "");
        assert_eq!(d.finish(), "\u{FFFD}");
    }
}
